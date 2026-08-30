use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct DspConfig {
    pub input_gain_db: f32,
    pub compressor_threshold_db: f32,
    pub compressor_ratio: f32,
    pub makeup_gain_db: f32,
    pub presence_amount: f32,
    pub clip_drive: f32,
    pub output_gain_db: f32,
    pub limiter_ceiling_db: f32,
    pub loudness_drive: f32,
    pub body_control: f32,
    pub air_amount: f32,
    pub deesser_amount: f32,
    pub density_amount: f32,
    pub noise_gate_amount: f32,
    pub competition_amount: f32,
    pub overdrive_amount: f32,
    pub transient_punch: f32,
    pub external_eq_mode: bool,
    pub external_eq_headroom_db: f32,
    pub bypass: bool,
    pub oversampling: u8,
}

impl Default for DspConfig {
    fn default() -> Self {
        Self {
            input_gain_db: 10.0,
            compressor_threshold_db: -20.0,
            compressor_ratio: 7.0,
            makeup_gain_db: 9.0,
            presence_amount: 0.55,
            clip_drive: 1.8,
            output_gain_db: 3.0,
            limiter_ceiling_db: -0.8,
            loudness_drive: 0.72,
            body_control: 0.58,
            air_amount: 0.44,
            deesser_amount: 0.52,
            density_amount: 0.66,
            noise_gate_amount: 0.35,
            competition_amount: 0.0,
            overdrive_amount: 0.0,
            transient_punch: 0.35,
            external_eq_mode: false,
            external_eq_headroom_db: 8.0,
            bypass: false,
            oversampling: 2,
        }
    }
}

pub struct VoiceDsp {
    cfg: DspConfig,
    env: f32,
    gate_env: f32,
    hp_x1: f32,
    hp_y1: f32,
    presence_lp: f32,
    body_lp: f32,
    mud_lp: f32,
    mud_env: f32,
    air_lp: f32,
    sibilance_lp: f32,
    sibilance_env: f32,
    harsh_env: f32,
    punch_env: f32,
    apo_peak_env: f32,
    apo_trim_db: f32,
    apo_detector_lp: f32,
    apo_low_env: f32,
    sample_rate: f32,
    limiter_delay: Vec<f32>,
    limiter_pos: usize,
    limiter_gain: f32,
    prev_sat_input: f32,
    last_comp_reduction_db: f32,
    last_limiter_reduction_db: f32,
    last_clip_activity: f32,
    last_dynamic_eq_activity: f32,
    last_deesser_reduction_db: f32,
    last_gate_reduction_db: f32,
    last_apo_trim_db: f32,
    last_apo_hot: f32,
    last_apo_bass_protection: f32,
}

impl VoiceDsp {
    pub fn new(sample_rate: f32, cfg: DspConfig) -> Self {
        let lookahead_samples = ((sample_rate * 0.0022).round() as usize).max(16);
        Self {
            cfg,
            env: 0.0,
            gate_env: 0.0,
            hp_x1: 0.0,
            hp_y1: 0.0,
            presence_lp: 0.0,
            body_lp: 0.0,
            mud_lp: 0.0,
            mud_env: 0.0,
            air_lp: 0.0,
            sibilance_lp: 0.0,
            sibilance_env: 0.0,
            harsh_env: 0.0,
            punch_env: 0.0,
            apo_peak_env: 0.0,
            apo_trim_db: 0.0,
            apo_detector_lp: 0.0,
            apo_low_env: 0.0,
            sample_rate,
            limiter_delay: vec![0.0; lookahead_samples],
            limiter_pos: 0,
            limiter_gain: 1.0,
            prev_sat_input: 0.0,
            last_comp_reduction_db: 0.0,
            last_limiter_reduction_db: 0.0,
            last_clip_activity: 0.0,
            last_dynamic_eq_activity: 0.0,
            last_deesser_reduction_db: 0.0,
            last_gate_reduction_db: 0.0,
            last_apo_trim_db: 0.0,
            last_apo_hot: 0.0,
            last_apo_bass_protection: 0.0,
        }
    }

    pub fn set_config(&mut self, cfg: DspConfig) { self.cfg = cfg; }
    pub fn compressor_reduction_db(&self) -> f32 { self.last_comp_reduction_db }
    pub fn limiter_reduction_db(&self) -> f32 { self.last_limiter_reduction_db }
    pub fn clip_activity(&self) -> f32 { self.last_clip_activity }
    pub fn dynamic_eq_activity(&self) -> f32 { self.last_dynamic_eq_activity }
    pub fn deesser_reduction_db(&self) -> f32 { self.last_deesser_reduction_db }
    pub fn gate_reduction_db(&self) -> f32 { self.last_gate_reduction_db }
    pub fn apo_input_trim_db(&self) -> f32 { self.last_apo_trim_db }
    pub fn apo_input_hot(&self) -> f32 { self.last_apo_hot }
    pub fn apo_bass_protection(&self) -> f32 { self.last_apo_bass_protection }

    #[inline] fn db_to_gain(db: f32) -> f32 { 10.0_f32.powf(db / 20.0) }
    #[inline] fn gain_to_db(gain: f32) -> f32 { 20.0 * gain.max(1.0e-9).log10() }
    #[inline] fn soft_clip(x: f32, drive: f32) -> f32 { let d=drive.max(1.0); (x*d).tanh()/d.tanh() }

    fn oversampled_saturation(&mut self, x: f32, drive: f32, stage2: f32) -> f32 {
        let os = self.cfg.oversampling.clamp(1, 4) as usize;
        if os == 1 { self.prev_sat_input=x; return Self::soft_clip(Self::soft_clip(x, drive), stage2); }
        let mut acc=0.0;
        for i in 1..=os {
            let t=i as f32/os as f32;
            let s=self.prev_sat_input + (x-self.prev_sat_input)*t;
            acc += Self::soft_clip(Self::soft_clip(s, drive), stage2);
        }
        self.prev_sat_input=x;
        acc/os as f32
    }

    fn lookahead_limit(&mut self, x: f32) -> f32 {
        let delayed=self.limiter_delay[self.limiter_pos];
        self.limiter_delay[self.limiter_pos]=x;
        self.limiter_pos=(self.limiter_pos+1)%self.limiter_delay.len();
        let ceiling=Self::db_to_gain(self.cfg.limiter_ceiling_db);
        let future_peak=self.limiter_delay.iter().fold(0.0_f32,|p,s|p.max(s.abs()));
        let target=if future_peak>ceiling { ceiling/future_peak.max(1e-9) } else {1.0};
        if target<self.limiter_gain { self.limiter_gain=target; } else {
            let release_ms=if self.limiter_gain<0.72 {104.0}else{54.0};
            let rel=(-1.0/(self.sample_rate*release_ms*0.001)).exp();
            self.limiter_gain=rel*self.limiter_gain+(1.0-rel)*target;
        }
        self.last_limiter_reduction_db=(-Self::gain_to_db(self.limiter_gain)).max(0.0);
        (delayed*self.limiter_gain).clamp(-ceiling,ceiling)
    }

    pub fn process_sample(&mut self, input: f32) -> f32 {
        if self.cfg.bypass {
            self.last_comp_reduction_db=0.0; self.last_limiter_reduction_db=0.0; self.last_clip_activity=0.0;
            self.last_dynamic_eq_activity=0.0; self.last_deesser_reduction_db=0.0; self.last_gate_reduction_db=0.0;
            self.last_apo_trim_db=0.0; self.last_apo_hot=0.0; self.last_apo_bass_protection=0.0;
            return input.clamp(-1.0,1.0);
        }

        let drive=self.cfg.loudness_drive.clamp(0.0,1.0);
        let density=self.cfg.density_amount.clamp(0.0,1.0);
        let competition=self.cfg.competition_amount.clamp(0.0,1.0);
        let overdrive=self.cfg.overdrive_amount.clamp(0.0,1.0);
        let punch=self.cfg.transient_punch.clamp(0.0,1.0);
        let apo=self.cfg.external_eq_mode;

        // APO INPUT GUARD 2.0 -------------------------------------------------
        // Equalizer APO commonly runs a floating-point chain above 0 dBFS. The old
        // implementation only subtracted a fixed amount, so Flaw's own input gain,
        // compressor makeup and saturation could immediately spend that headroom again.
        // This guard measures the real incoming peak and reserves enough headroom for
        // both the external EQ and Flaw's own preamp before any nonlinear DSP runs.
        let peak_attack=(-1.0/(self.sample_rate*0.0015)).exp();
        let peak_release=(-1.0/(self.sample_rate*0.850)).exp();
        let raw_peak=input.abs();
        let pc=if raw_peak>self.apo_peak_env {peak_attack}else{peak_release};
        self.apo_peak_env=pc*self.apo_peak_env+(1.0-pc)*raw_peak;

        let manual_headroom=if apo { self.cfg.external_eq_headroom_db.clamp(0.0,18.0) } else {0.0};
        let user_input_db=if apo { self.cfg.input_gain_db.clamp(-12.0,7.0) } else { self.cfg.input_gain_db };
        let extra_preamp=(drive*8.2+competition*2.6+overdrive*1.4) * if apo {0.20}else{1.0};
        let preamp_db=user_input_db+extra_preamp;
        let projected_peak=self.apo_peak_env*Self::db_to_gain(preamp_db-manual_headroom);
        let apo_target_peak=0.52; // about -5.7 dBFS before dynamics; room for transients and EQ overshoot.
        let required_auto_trim=if apo && projected_peak>apo_target_peak {
            (Self::gain_to_db(projected_peak/apo_target_peak)).clamp(0.0,18.0)
        } else {0.0};
        let trim_attack=(-1.0/(self.sample_rate*0.0030)).exp();
        let trim_release=(-1.0/(self.sample_rate*1.250)).exp();
        let tc=if required_auto_trim>self.apo_trim_db {trim_attack}else{trim_release};
        self.apo_trim_db=if apo {tc*self.apo_trim_db+(1.0-tc)*required_auto_trim}else{0.0};
        let total_apo_trim=if apo {manual_headroom+self.apo_trim_db}else{0.0};
        self.last_apo_trim_db=total_apo_trim;
        self.last_apo_hot=if apo {
            (((projected_peak-0.58)/0.72).clamp(0.0,1.0)).max(((raw_peak-0.985)/0.50).clamp(0.0,1.0))
        } else {0.0};

        let mut x=input*Self::db_to_gain(preamp_db-total_apo_trim);

        // 72 Hz rumble filter.
        let rc=1.0/(std::f32::consts::TAU*72.0); let dt=1.0/self.sample_rate; let alpha=rc/(rc+dt);
        let hp=alpha*(self.hp_y1+x-self.hp_x1); self.hp_x1=x; self.hp_y1=hp; x=hp;

        // APO bass-aware detector. Only the detector is de-weighted: the audible bass
        // stays intact, but a bass-heavy external set can no longer pin the compressor.
        let apo_low_coeff=(-std::f32::consts::TAU*235.0/self.sample_rate).exp();
        self.apo_detector_lp=apo_low_coeff*self.apo_detector_lp+(1.0-apo_low_coeff)*x;
        let low_ratio=(self.apo_detector_lp.abs()/(x.abs()+0.0008)).clamp(0.0,1.6);
        let low_target=((low_ratio-0.42)/0.72).clamp(0.0,1.0);
        let low_smooth=(-1.0/(self.sample_rate*0.090)).exp();
        self.apo_low_env=low_smooth*self.apo_low_env+(1.0-low_smooth)*low_target;
        self.last_apo_bass_protection=if apo {self.apo_low_env}else{0.0};

        // Smooth adaptive gate. APO mode backs it off to preserve tails altered by EQ.
        let gate_amt=self.cfg.noise_gate_amount.clamp(0.0,1.0)*if apo {0.62}else{1.0};
        let gate_a=(-1.0/(self.sample_rate*0.004)).exp(); let gate_r=(-1.0/(self.sample_rate*0.110)).exp();
        let detector=x.abs(); let gc=if detector>self.gate_env {gate_a}else{gate_r};
        self.gate_env=gc*self.gate_env+(1.0-gc)*detector;
        let gate_threshold=0.0025+gate_amt*0.0105;
        let gate_gain=if self.gate_env<gate_threshold { (0.16+(self.gate_env/gate_threshold)*0.84).powf(1.0+gate_amt*1.8) } else {1.0};
        self.last_gate_reduction_db=(-Self::gain_to_db(gate_gain)).max(0.0); x*=gate_gain;
        let dry=x;

        // Main compressor. APO mode uses a bass-deweighted sidechain and a much lower
        // maximum ratio so the external EQ character survives instead of sounding covered.
        let detector_sample=if apo {
            let deweighted=x-self.apo_detector_lp*(0.48+self.apo_low_env*0.28);
            deweighted.abs().max(x.abs()*0.26)
        } else {x.abs()};
        let coeff_a=(-1.0/(self.sample_rate*if apo {0.0032}else{0.0015})).exp();
        let coeff_r=(-1.0/(self.sample_rate*if apo {0.120}else{0.068})).exp();
        let c=if detector_sample>self.env {coeff_a}else{coeff_r}; self.env=c*self.env+(1.0-c)*detector_sample;
        let threshold=self.cfg.compressor_threshold_db-drive*6.4-density*2.2-competition*3.2 + if apo {6.0+self.apo_low_env*1.8}else{0.0};
        let ratio_base=self.cfg.compressor_ratio+drive*5.5+density*2.5+competition*3.8;
        let ratio=if apo {(ratio_base*0.48).clamp(1.35,4.8)}else{ratio_base.max(1.0)};
        let env_db=Self::gain_to_db(self.env); let mut gr=0.0;
        if env_db>threshold { let over=env_db-threshold; let reduced=over/ratio; gr=(over-reduced).max(0.0); x*=Self::db_to_gain(-gr); }
        self.last_comp_reduction_db=gr;
        let makeup_db=if apo {
            self.cfg.makeup_gain_db*0.42+drive*2.25+density*0.45+competition*0.30
        } else {
            self.cfg.makeup_gain_db+drive*7.2+density*1.8+competition*1.6
        };
        x*=Self::db_to_gain(makeup_db);
        let wet=if apo {(0.43+drive*0.10+density*0.08).clamp(0.38,0.62)}else{(0.52+drive*0.22+density*0.22+competition*0.04).clamp(0.0,0.96)};
        x=x*wet+dry*(1.0-wet);

        // Competition pressure is deliberately reduced under APO: loudness is recovered
        // later with clean gain rather than another nonlinear layer.
        let pressure_amt=competition*if apo {0.42}else{1.0};
        let pressure_drive=1.0+pressure_amt*2.2;
        let pressure=Self::soft_clip(x*Self::db_to_gain(pressure_amt*4.0),pressure_drive);
        x=x*(1.0-pressure_amt*0.26)+pressure*(pressure_amt*0.26);

        // Transient recovery restores articulation after compression; APO gets a little
        // more recovery because this is exactly the detail that was being masked.
        let pa=(-1.0/(self.sample_rate*0.0010)).exp();
        let pr=(-1.0/(self.sample_rate*0.045)).exp();
        let pc=if dry.abs()>self.punch_env {pa}else{pr};
        self.punch_env=pc*self.punch_env+(1.0-pc)*dry.abs();
        let transient=(dry.abs()-self.punch_env).max(0.0)*dry.signum();
        let transient_guard=(1.0-(self.last_comp_reduction_db/24.0).clamp(0.0,1.0)*0.18).clamp(0.82,1.0);
        let apo_articulation=if apo {1.35}else{1.0};
        x+=transient*punch*(0.20+pressure_amt*0.42)*transient_guard*apo_articulation;

        // Dynamic low-mid cleanup + controlled body. APO mode avoids adding more body on
        // top of EQ sets that often already carry large 100-500 Hz boosts.
        let mud_coeff=(-std::f32::consts::TAU*330.0/self.sample_rate).exp(); self.mud_lp=mud_coeff*self.mud_lp+(1.0-mud_coeff)*x;
        let ma=(-1.0/(self.sample_rate*0.012)).exp(); let mr=(-1.0/(self.sample_rate*0.140)).exp();
        let mc=if self.mud_lp.abs()>self.mud_env {ma}else{mr}; self.mud_env=mc*self.mud_env+(1.0-mc)*self.mud_lp.abs();
        let mud_over=((self.mud_env-0.16)/0.52).clamp(0.0,1.0);
        let mud_red=mud_over*self.cfg.body_control.clamp(0.0,1.0)*(0.14+drive*0.10+competition*0.055)*if apo {0.52}else{1.0};
        x-=self.mud_lp*mud_red; self.last_dynamic_eq_activity=(mud_red/0.24).clamp(0.0,1.0);
        let body_coeff=(-std::f32::consts::TAU*520.0/self.sample_rate).exp(); self.body_lp=body_coeff*self.body_lp+(1.0-body_coeff)*x;
        let body_add=if apo {0.008+density*0.018}else{0.035+density*0.055};
        x+=self.body_lp*body_add;

        // Presence / HarshGuard. In APO mode harshness protection becomes transparent:
        // it only catches true spikes and no longer shaves the entire clarity band.
        let lp_coeff=(-std::f32::consts::TAU*3400.0/self.sample_rate).exp(); self.presence_lp=lp_coeff*self.presence_lp+(1.0-lp_coeff)*x;
        let detail=x-self.presence_lp; let ha=(-1.0/(self.sample_rate*0.0025)).exp(); let hr=(-1.0/(self.sample_rate*0.075)).exp();
        let hc=if detail.abs()>self.harsh_env {ha}else{hr}; self.harsh_env=hc*self.harsh_env+(1.0-hc)*detail.abs();
        let harsh_target=((0.38-competition*0.035).max(0.32)) + if apo {0.18}else{0.0};
        let harsh_guard=if self.harsh_env>harsh_target {(harsh_target/self.harsh_env).clamp(if apo {0.86}else{0.46},1.0)}else{1.0};
        let presence=(self.cfg.presence_amount+drive*0.25+if apo {0.06}else{0.0}).clamp(0.0,1.55);
        x+=detail*presence*harsh_guard;

        // De-esser. APO sets already sculpt this band, so only obvious sibilant spikes are reduced.
        let sib_coeff=(-std::f32::consts::TAU*5200.0/self.sample_rate).exp(); self.sibilance_lp=sib_coeff*self.sibilance_lp+(1.0-sib_coeff)*x;
        let sib=x-self.sibilance_lp; let sa=(-1.0/(self.sample_rate*0.0012)).exp(); let sr=(-1.0/(self.sample_rate*0.060)).exp();
        let sc=if sib.abs()>self.sibilance_env {sa}else{sr}; self.sibilance_env=sc*self.sibilance_env+(1.0-sc)*sib.abs();
        let sib_over=((self.sibilance_env-(if apo {0.27}else{0.20}))/(if apo {0.62}else{0.48})).clamp(0.0,1.0);
        let effective_deesser=self.cfg.deesser_amount.clamp(0.0,1.0)*if apo {0.20}else{1.0};
        let deess_gain=1.0-sib_over*effective_deesser*0.62;
        x=self.sibilance_lp+sib*deess_gain; self.last_deesser_reduction_db=(-Self::gain_to_db(deess_gain)).max(0.0);

        // Air enhancer after de-essing. A small APO clarity recovery offsets the softer dynamics.
        let air_coeff=(-std::f32::consts::TAU*7500.0/self.sample_rate).exp(); self.air_lp=air_coeff*self.air_lp+(1.0-air_coeff)*x;
        let air_scale=if apo {1.16}else{1.0};
        x+=(x-self.air_lp)*self.cfg.air_amount.clamp(0.0,1.0)*(0.12+drive*0.10)*air_scale;

        // Saturation. This was the biggest destructive interaction with hot APO sets.
        // APO mode keeps only a light density stage and recovers loudness with clean gain.
        let smart_clean=(1.0-(self.last_limiter_reduction_db/12.0).clamp(0.0,1.0)*(0.12+pressure_amt*0.18)).clamp(if apo {0.82}else{0.68},1.0);
        x*=smart_clean;
        let pre_clip=x;
        let (sat_drive,sat2,post_sat_db)=if apo {
            (1.0+(self.cfg.clip_drive-1.0)*0.28+drive*0.38+density*0.10+pressure_amt*0.12+overdrive*0.16,
             1.04+drive*0.16+overdrive*0.08,
             drive*0.85+density*0.18+pressure_amt*0.22+overdrive*0.20)
        } else {
            (self.cfg.clip_drive+drive*1.45+density*0.35+competition*0.62+overdrive*1.28,
             1.10+drive*0.68+overdrive*0.38,
             drive*3.5+density*0.7+competition*1.2+overdrive*1.8)
        };
        x=self.oversampled_saturation(x,sat_drive,sat2)*Self::db_to_gain(post_sat_db);
        self.last_clip_activity=((pre_clip.abs()-x.abs()).abs()*if apo {1.0}else{1.55}).clamp(0.0,1.0);
        if overdrive>0.0 {
            let od=overdrive*if apo {0.22}else{1.0};
            x=Self::soft_clip(x,1.0+od*2.4);
        }

        // Clean loudness recovery: restore only part of the reserved APO headroom, then use
        // limiter feedback to stop sustained CleanGuard crushing. This keeps it loud without
        // returning to the flattened waveform seen in the user's Equalizer APO recording.
        let apo_recovery_db=if apo {(total_apo_trim*0.34).min(4.2)}else{0.0};
        let limiter_servo_db=if apo {(self.last_limiter_reduction_db-2.2).max(0.0)*0.58}else{0.0};
        let base_output_db=if apo {self.cfg.output_gain_db*0.72+drive*0.75+pressure_amt*0.30}else{self.cfg.output_gain_db+drive*1.8+competition*0.8};
        x*=Self::db_to_gain(base_output_db+apo_recovery_db-limiter_servo_db);
        self.lookahead_limit(x)
    }
}
