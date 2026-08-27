#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod dsp;
mod license;
use cpal::{SampleFormat, StreamConfig, SupportedStreamConfig};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use dsp::{DspConfig, VoiceDsp};
use ringbuf::{HeapRb, traits::{Consumer, Producer, Split}};
use serde::Serialize;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::time::{Duration, Instant};
use tauri::{Manager, State};
use license::LicenseState;


#[derive(Clone, Serialize)] struct SystemDiagnostics {
    app_version:String, os:String, arch:String, native_audio:bool, input_devices:usize, output_devices:usize, engine_running:bool, sample_rate:u32, latency_ms:f32, underruns:u64, clip_count:u64
}
#[derive(Clone, Serialize)] struct UpdateStatus { configured:bool, enabled:bool, channel:String, current_version:String, message:String }
#[derive(Clone, Serialize)] struct UpdateCheck { configured:bool, available:bool, version:String, notes:String, message:String }
#[tauri::command] fn get_system_diagnostics(state:State<'_,EngineState>)->Result<SystemDiagnostics,String>{
    let host=cpal::default_host();
    let inputs=host.input_devices().map(|d|d.count()).unwrap_or(0);
    let outputs=host.output_devices().map(|d|d.count()).unwrap_or(0);
    let m=*state.metrics.lock().map_err(|_|"Metrics state lock failed")?;
    Ok(SystemDiagnostics{app_version:env!("CARGO_PKG_VERSION").into(),os:std::env::consts::OS.into(),arch:std::env::consts::ARCH.into(),native_audio:true,input_devices:inputs,output_devices:outputs,engine_running:state.running.load(Ordering::Relaxed),sample_rate:m.sample_rate,latency_ms:m.latency_ms,underruns:m.underruns,clip_count:m.clip_count})
}
#[tauri::command] fn get_update_status()->UpdateStatus{
    UpdateStatus{configured:true,enabled:true,channel:"stable".into(),current_version:env!("CARGO_PKG_VERSION").into(),message:"Signed Tauri updater ready".into()}
}
#[tauri::command] async fn check_for_updates(app:tauri::AppHandle)->Result<UpdateCheck,String>{
    #[cfg(desktop)]
    {
        use tauri_plugin_updater::UpdaterExt;
        let updater=app.updater().map_err(|e|format!("Updater initialization failed: {e}"))?;
        match updater.check().await.map_err(|e|format!("Update check failed: {e}"))? {
            Some(update)=>Ok(UpdateCheck{configured:true,available:true,version:update.version.clone(),notes:update.body.clone().unwrap_or_default(),message:format!("Flaw Loud {} is available",update.version)}),
            None=>Ok(UpdateCheck{configured:true,available:false,version:String::new(),notes:String::new(),message:"Flaw Loud is up to date".into()})
        }
    }
    #[cfg(not(desktop))]
    { Err("Updater is only available on desktop builds.".into()) }
}
#[tauri::command] async fn install_update(app:tauri::AppHandle)->Result<String,String>{
    #[cfg(desktop)]
    {
        use tauri_plugin_updater::UpdaterExt;
        let updater=app.updater().map_err(|e|format!("Updater initialization failed: {e}"))?;
        let update=updater.check().await.map_err(|e|format!("Update check failed: {e}"))?.ok_or_else(||"Flaw Loud is already up to date.".to_string())?;
        update.download_and_install(|_,_|{},||{}).await.map_err(|e|format!("Update installation failed: {e}"))?;
        app.restart();
    }
    #[cfg(not(desktop))]
    { Err("Updater is only available on desktop builds.".into()) }
}
#[tauri::command] fn export_support_report(app:tauri::AppHandle,state:State<'_,EngineState>,input_name:String,output_name:String,profile:String,quality:String,ceiling:f32,stream_mode:bool,theme_fx:String,stream_hotkey:String,unload_hotkey:String,latency_target:String,hotkeys_ready:bool)->Result<String,String>{
    let host=cpal::default_host(); let m=*state.metrics.lock().map_err(|_|"Metrics state lock failed")?;
    let report=serde_json::json!({
      "product":"Flaw Loud","version":env!("CARGO_PKG_VERSION"),"publisher":"Bnet",
      "system":{"os":std::env::consts::OS,"arch":std::env::consts::ARCH},
      "audio":{"engine_running":state.running.load(Ordering::Relaxed),"sample_rate":m.sample_rate,"latency_ms":m.latency_ms,"underruns":m.underruns,"clip_count":m.clip_count,"input_device":input_name,"output_device":output_name,"input_devices_detected":host.input_devices().map(|d|d.count()).unwrap_or(0),"output_devices_detected":host.output_devices().map(|d|d.count()).unwrap_or(0)},
      "configuration":{"profile":profile,"quality":quality,"ceiling_dbfs":ceiling,"stream_mode":stream_mode,"theme_fx":theme_fx,"latency_target":latency_target,"stream_hotkey":stream_hotkey,"unload_hotkey":unload_hotkey,"global_hotkeys_ready":hotkeys_ready},
      "privacy":"No license key, developer password, HWID, microphone audio, or personal files are included."
    });
    let dir=app.path().app_data_dir().map_err(|e|e.to_string())?.join("support"); std::fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
    let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e|e.to_string())?.as_secs(); let path=dir.join(format!("Flaw_Loud_Support_{stamp}.json"));
    std::fs::write(&path,serde_json::to_vec_pretty(&report).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}
#[tauri::command] fn minimize_to_tray(window:tauri::WebviewWindow)->Result<(),String>{window.hide().map_err(|e|e.to_string())}
#[tauri::command] fn apply_stream_mode(window:tauri::WebviewWindow,enabled:bool)->Result<(),String>{
    window.set_skip_taskbar(true).map_err(|e|e.to_string())?;
    window.set_content_protected(enabled).map_err(|e|e.to_string())?;
    Ok(())
}

#[tauri::command] fn unload_app(app:tauri::AppHandle,state:State<'_,EngineState>)->Result<(),String>{
    state.stop.store(true,Ordering::SeqCst);
    state.running.store(false,Ordering::SeqCst);
    app.exit(0);
    Ok(())
}
#[tauri::command] fn toggle_stream_window(window:tauri::WebviewWindow)->Result<(),String>{
    let visible=window.is_visible().map_err(|e|e.to_string())?;
    if visible { window.hide().map_err(|e|e.to_string())?; }
    else { window.show().map_err(|e|e.to_string())?; window.set_focus().map_err(|e|e.to_string())?; }
    Ok(())
}

#[derive(Clone, Serialize)] struct AudioDeviceInfo { id:String, name:String, is_default:bool }
#[derive(Serialize)] struct AudioDevices { inputs:Vec<AudioDeviceInfo>, outputs:Vec<AudioDeviceInfo> }
#[derive(Clone, Copy, Serialize, Default)] struct EngineMetrics {
    input_peak:f32, output_peak:f32, input_rms:f32, output_rms:f32, underruns:u64, sample_rate:u32, latency_ms:f32,
    gain_reduction_db:f32, limiter_reduction_db:f32, clip_activity:f32, dynamic_eq_activity:f32, deesser_reduction_db:f32,
    gate_reduction_db:f32, clip_count:u64,
}
#[derive(Serialize)] struct VisualFrame { waveform:Vec<f32>, bands:Vec<f32> }

struct EngineState {
    running:Arc<AtomicBool>, stop:Arc<AtomicBool>, metrics:Arc<Mutex<EngineMetrics>>, config:Arc<Mutex<DspConfig>>,
    visual:Arc<Mutex<Vec<f32>>>,
}
impl Default for EngineState { fn default()->Self { Self{running:Arc::new(AtomicBool::new(false)),stop:Arc::new(AtomicBool::new(false)),metrics:Arc::new(Mutex::new(EngineMetrics::default())),config:Arc::new(Mutex::new(DspConfig::default())),visual:Arc::new(Mutex::new(vec![0.0;256]))} } }

fn device_name(d:&cpal::Device)->String{d.description().map(|x|x.name().to_string()).unwrap_or_else(|_|"Unknown audio device".into())}
fn device_id(d:&cpal::Device)->Option<String>{d.id().ok().map(|x|x.to_string())}
fn find_device(host:&cpal::Host,id:&str,input:bool)->Result<cpal::Device,String>{
    let v=if input{host.input_devices().map_err(|e|e.to_string())?.collect::<Vec<_>>()}else{host.output_devices().map_err(|e|e.to_string())?.collect::<Vec<_>>()};
    v.into_iter().find(|d|device_id(d).as_deref()==Some(id)).ok_or_else(||"Selected audio device is no longer available.".into())
}
fn f32_config_at(device:&cpal::Device,input:bool,rate:u32)->Option<SupportedStreamConfig>{
    let ranges=if input{device.supported_input_configs().ok()?.collect::<Vec<_>>()}else{device.supported_output_configs().ok()?.collect::<Vec<_>>()};
    ranges.into_iter().filter(|r|r.sample_format()==SampleFormat::F32).filter_map(|r|r.try_with_sample_rate(rate)).min_by_key(|c|{let ch=c.channels();if ch==1{0}else if ch==2{1}else{2+ch as u16}})
}

#[tauri::command] fn list_audio_devices()->Result<AudioDevices,String>{
    let host=cpal::default_host(); let default_in=host.default_input_device().and_then(|d|device_id(&d)); let default_out=host.default_output_device().and_then(|d|device_id(&d));
    let inputs=host.input_devices().map_err(|e|e.to_string())?.filter_map(|d|device_id(&d).map(|id|AudioDeviceInfo{name:device_name(&d),is_default:default_in.as_deref()==Some(id.as_str()),id})).collect();
    let outputs=host.output_devices().map_err(|e|e.to_string())?.filter_map(|d|device_id(&d).map(|id|AudioDeviceInfo{name:device_name(&d),is_default:default_out.as_deref()==Some(id.as_str()),id})).collect();
    Ok(AudioDevices{inputs,outputs})
}
#[tauri::command] fn set_dsp_config(config:DspConfig,state:State<'_,EngineState>)->Result<(),String>{*state.config.lock().map_err(|_|"DSP state lock failed")?=config;Ok(())}
#[tauri::command] fn get_engine_metrics(state:State<'_,EngineState>)->Result<EngineMetrics,String>{let mut m=*state.metrics.lock().map_err(|_|"Metrics state lock failed")?;if !state.running.load(Ordering::Relaxed){m.input_peak=0.0;m.output_peak=0.0;m.input_rms=0.0;m.output_rms=0.0;m.gain_reduction_db=0.0;m.limiter_reduction_db=0.0;m.clip_activity=0.0;m.dynamic_eq_activity=0.0;m.deesser_reduction_db=0.0;m.gate_reduction_db=0.0;}Ok(m)}
#[tauri::command] fn get_visual_frame(state:State<'_,EngineState>)->Result<VisualFrame,String>{
    let wave=state.visual.lock().map_err(|_|"Visual state lock failed")?.clone();
    let n=wave.len().max(1); let mut bands=Vec::with_capacity(32);
    for b in 0..32 { let k=1+b*3; let mut re=0.0f32; let mut im=0.0f32; for (i,s) in wave.iter().enumerate(){let a=std::f32::consts::TAU*(k as f32)*(i as f32)/(n as f32);re+=*s*a.cos();im-=*s*a.sin();} bands.push(((re*re+im*im).sqrt()/(n as f32)*5.0).min(1.0)); }
    Ok(VisualFrame{waveform:wave,bands})
}
#[tauri::command] fn stop_engine(state:State<'_,EngineState>)->Result<(),String>{state.stop.store(true,Ordering::SeqCst);Ok(())}

#[tauri::command] fn start_engine(input_id:String,output_id:String,latency_target:Option<String>,state:State<'_,EngineState>)->Result<String,String>{
    if state.running.load(Ordering::SeqCst){return Ok("Engine is already running".into())}
    state.stop.store(false,Ordering::SeqCst); let running=state.running.clone(); let stop=state.stop.clone(); let metrics=state.metrics.clone(); let config=state.config.clone(); let visual=state.visual.clone(); let (tx,rx)=std::sync::mpsc::sync_channel::<Result<String,String>>(1);
    std::thread::spawn(move||{
        let result=(||->Result<(),String>{
            let host=cpal::default_host(); let input_device=find_device(&host,&input_id,true)?; let output_device=find_device(&host,&output_id,false)?;
            let mut chosen=None; for rate in [48_000u32,44_100u32]{if let(Some(ic),Some(oc))=(f32_config_at(&input_device,true,rate),f32_config_at(&output_device,false,rate)){chosen=Some((rate,ic,oc));break}}
            let(rate,input_supported,output_supported)=chosen.ok_or_else(||"Input and output need a common 48 kHz or 44.1 kHz F32 format. Try setting both devices to 48 kHz in Windows Sound settings.".to_string())?;
            let input_channels=input_supported.channels() as usize; let output_channels=output_supported.channels() as usize; let input_cfg:StreamConfig=input_supported.config(); let output_cfg:StreamConfig=output_supported.config();
            let target=latency_target.unwrap_or_else(||"Balanced".into()); let(cap_ms,delay_ms,reported_ms)=match target.as_str(){"Lowest"=>(55usize,7.0f32,9.0f32),"Safe"=>(130usize,24.0f32,27.0f32),_=>(80usize,12.0f32,14.0f32)}; let capacity_frames=(rate as usize/1000)*cap_ms; let rb=HeapRb::<f32>::new(capacity_frames.max(2048)); let(mut producer,mut consumer)=rb.split(); let delay_frames=((rate as f32*(delay_ms/1000.0))as usize).min(capacity_frames/2); for _ in 0..delay_frames { let _ = producer.try_push(0.0); }
            let input_metrics=metrics.clone(); let input_config=config.clone(); let input_visual=visual.clone(); let mut dsp=VoiceDsp::new(rate as f32,*input_config.lock().map_err(|_|"DSP state lock failed")?); let mut last_cfg_read=Instant::now(); let mut visual_buf=Vec::<f32>::with_capacity(256);
            let input_stream=input_device.build_input_stream(input_cfg,move|data:&[f32],_|{
                if last_cfg_read.elapsed()>=Duration::from_millis(20){if let Ok(c)=input_config.try_lock(){dsp.set_config(*c)}last_cfg_read=Instant::now()}
                let mut peak_in=0.0f32;let mut peak_out=0.0f32;let mut sum_in=0.0f32;let mut sum_out=0.0f32;let mut frames=0u32;let mut max_gr=0.0f32;let mut max_lgr=0.0f32;let mut max_clip=0.0f32;let mut max_deq=0.0f32;let mut max_ds=0.0f32;let mut max_gate=0.0f32;let mut clips=0u64;
                for frame in data.chunks(input_channels.max(1)){let mono=frame.iter().copied().sum::<f32>()/frame.len().max(1) as f32;let processed=dsp.process_sample(mono);peak_in=peak_in.max(mono.abs());peak_out=peak_out.max(processed.abs());sum_in+=mono*mono;sum_out+=processed*processed;frames+=1;max_gr=max_gr.max(dsp.compressor_reduction_db());max_lgr=max_lgr.max(dsp.limiter_reduction_db());max_clip=max_clip.max(dsp.clip_activity());max_deq=max_deq.max(dsp.dynamic_eq_activity());max_ds=max_ds.max(dsp.deesser_reduction_db());max_gate=max_gate.max(dsp.gate_reduction_db());if processed.abs()>0.995{clips+=1}let _=producer.try_push(processed);visual_buf.push(processed);if visual_buf.len()>=256{if let Ok(mut v)=input_visual.try_lock(){v.copy_from_slice(&visual_buf[..256]);}visual_buf.clear();}}
                if let Ok(mut m)=input_metrics.try_lock(){let fi=frames.max(1)as f32;m.input_peak=m.input_peak*0.72+peak_in*0.28;m.output_peak=m.output_peak*0.72+peak_out*0.28;m.input_rms=m.input_rms*0.75+(sum_in/fi).sqrt()*0.25;m.output_rms=m.output_rms*0.75+(sum_out/fi).sqrt()*0.25;m.gain_reduction_db=m.gain_reduction_db*0.62+max_gr*0.38;m.limiter_reduction_db=m.limiter_reduction_db*0.62+max_lgr*0.38;m.clip_activity=m.clip_activity*0.55+max_clip*0.45;m.dynamic_eq_activity=m.dynamic_eq_activity*0.62+max_deq*0.38;m.deesser_reduction_db=m.deesser_reduction_db*0.62+max_ds*0.38;m.gate_reduction_db=m.gate_reduction_db*0.62+max_gate*0.38;m.clip_count=m.clip_count.saturating_add(clips);}
            },move|err|eprintln!("Flaw Loud input stream: {err}"),None).map_err(|e|format!("Input stream failed: {e}"))?;
            let output_metrics=metrics.clone(); let output_stream=output_device.build_output_stream(output_cfg,move|data:&mut[f32],_|{for frame in data.chunks_mut(output_channels.max(1)){let sample=match consumer.try_pop(){Some(v)=>v,None=>{if let Ok(mut m)=output_metrics.try_lock(){m.underruns+=1}0.0}};frame.fill(sample)}},move|err|eprintln!("Flaw Loud output stream: {err}"),None).map_err(|e|format!("Output stream failed: {e}"))?;
            input_stream.play().map_err(|e|format!("Could not start microphone: {e}"))?;output_stream.play().map_err(|e|format!("Could not start output: {e}"))?;
            if let Ok(mut m)=metrics.lock(){m.sample_rate=rate;m.latency_ms=reported_ms;m.underruns=0;m.clip_count=0}running.store(true,Ordering::SeqCst);let _=tx.send(Ok(format!("{} → {} @ {} Hz",device_name(&input_device),device_name(&output_device),rate)));
            while !stop.load(Ordering::SeqCst){std::thread::sleep(Duration::from_millis(40))}drop(input_stream);drop(output_stream);running.store(false,Ordering::SeqCst);Ok(())
        })(); if let Err(err)=result{running.store(false,Ordering::SeqCst);let _=tx.send(Err(err));}
    });
    rx.recv_timeout(Duration::from_secs(6)).map_err(|_|"Audio engine startup timed out".to_string())?
}
fn main(){
    let mut builder=tauri::Builder::default();
    #[cfg(desktop)] { builder=builder.plugin(tauri_plugin_global_shortcut::Builder::new().build()).plugin(tauri_plugin_updater::Builder::new().build()); }
    builder
      .setup(|app|{
        #[cfg(desktop)] {
          use tauri::tray::TrayIconBuilder;
          let mut tray=TrayIconBuilder::with_id("flaw-loud").tooltip("Flaw Loud — engine active · use your Show / Hide hotkey");
          if let Some(icon)=app.default_window_icon(){ tray=tray.icon(icon.clone()); }
          tray.build(app)?;
        }
        Ok(())
      })
      .manage(EngineState::default()).manage(LicenseState::default())
      .invoke_handler(tauri::generate_handler![list_audio_devices,start_engine,stop_engine,set_dsp_config,get_engine_metrics,get_visual_frame,get_system_diagnostics,get_update_status,check_for_updates,install_update,export_support_report,minimize_to_tray,apply_stream_mode,toggle_stream_window,unload_app,license::get_license_status,license::activate_license,license::restore_saved_license,license::use_developer_preview,license::logout_license])
      .run(tauri::generate_context!()).expect("error while running Flaw Loud");
}
