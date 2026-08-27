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
        }
    }

    pub fn set_config(&mut self, cfg: DspConfig) { self.cfg = cfg; }
    pub fn compressor_reduction_db(&self) -> f32 { self.last_comp_reduction_db }
    pub fn limiter_reduction_db(&self) -> f32 { self.last_limiter_reduction_db }
    pub fn clip_activity(&self) -> f32 { self.last_clip_activity }
    pub fn dynamic_eq_activity(&self) -> f32 { self.last_dynamic_eq_activity }
    pub fn deesser_reduction_db(&self) -> f32 { self.last_deesser_reduction_db }
    pub fn gate_reduction_db(&self) -> f32 { self.last_gate_reduction_db }

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
            return input.clamp(-1.0,1.0);
        }
        let drive=self.cfg.loudness_drive.clamp(0.0,1.0);
        let density=self.cfg.density_amount.clamp(0.0,1.0);
        let competition=self.cfg.competition_amount.clamp(0.0,1.0);
        let overdrive=self.cfg.overdrive_amount.clamp(0.0,1.0);
        let punch=self.cfg.transient_punch.clamp(0.0,1.0);
        let extra_preamp=drive*8.2+competition*2.6+overdrive*1.4;
        let mut x=input*Self::db_to_gain(self.cfg.input_gain_db+extra_preamp);

        // 72 Hz rumble filter.
        let rc=1.0/(std::f32::consts::TAU*72.0); let dt=1.0/self.sample_rate; let alpha=rc/(rc+dt);
        let hp=alpha*(self.hp_y1+x-self.hp_x1); self.hp_x1=x; self.hp_y1=hp; x=hp;

        // Smooth adaptive gate. It never hard-mutes speech tails.
        let gate_amt=self.cfg.noise_gate_amount.clamp(0.0,1.0);
        let gate_a=(-1.0/(self.sample_rate*0.004)).exp(); let gate_r=(-1.0/(self.sample_rate*0.110)).exp();
        let detector=x.abs(); let gc=if detector>self.gate_env {gate_a}else{gate_r};
        self.gate_env=gc*self.gate_env+(1.0-gc)*detector;
        let gate_threshold=0.0025+gate_amt*0.0105;
        let gate_gain=if self.gate_env<gate_threshold { (0.16+(self.gate_env/gate_threshold)*0.84).powf(1.0+gate_amt*1.8) } else {1.0};
        self.last_gate_reduction_db=(-Self::gain_to_db(gate_gain)).max(0.0); x*=gate_gain;
        let dry=x;

        // Main compressor.
        let coeff_a=(-1.0/(self.sample_rate*0.0015)).exp(); let coeff_r=(-1.0/(self.sample_rate*0.068)).exp();
        let c=if x.abs()>self.env {coeff_a}else{coeff_r}; self.env=c*self.env+(1.0-c)*x.abs();
        let threshold=self.cfg.compressor_threshold_db-drive*6.4-density*2.2-competition*3.2;
        let ratio=(self.cfg.compressor_ratio+drive*5.5+density*2.5+competition*3.8).max(1.0);
        let env_db=Self::gain_to_db(self.env); let mut gr=0.0;
        if env_db>threshold { let over=env_db-threshold; let reduced=over/ratio; gr=(over-reduced).max(0.0); x*=Self::db_to_gain(-gr); }
        self.last_comp_reduction_db=gr; x*=Self::db_to_gain(self.cfg.makeup_gain_db+drive*7.2+density*1.8+competition*1.6);

        let wet=(0.52+drive*0.22+density*0.22+competition*0.04).clamp(0.0,0.96); x=x*wet+dry*(1.0-wet);

        // Competition parallel pressure: dense center without fully crushing articulation.
        let pressure_drive=1.0+competition*2.2;
        let pressure=Self::soft_clip(x*Self::db_to_gain(competition*4.0),pressure_drive);
        x=x*(1.0-competition*0.26)+pressure*(competition*0.26);

        // Transient recovery gives consonants and attacks back after heavy compression.
        let pa=(-1.0/(self.sample_rate*0.0010)).exp();
        let pr=(-1.0/(self.sample_rate*0.045)).exp();
        let pc=if dry.abs()>self.punch_env {pa}else{pr};
        self.punch_env=pc*self.punch_env+(1.0-pc)*dry.abs();
        let transient=(dry.abs()-self.punch_env).max(0.0)*dry.signum();
        let transient_guard=(1.0-(self.last_comp_reduction_db/24.0).clamp(0.0,1.0)*0.18).clamp(0.82,1.0); x+=transient*punch*(0.20+competition*0.42)*transient_guard;

        // Dynamic low-mid cleanup + controlled body.
        let mud_coeff=(-std::f32::consts::TAU*330.0/self.sample_rate).exp(); self.mud_lp=mud_coeff*self.mud_lp+(1.0-mud_coeff)*x;
        let ma=(-1.0/(self.sample_rate*0.012)).exp(); let mr=(-1.0/(self.sample_rate*0.140)).exp();
        let mc=if self.mud_lp.abs()>self.mud_env {ma}else{mr}; self.mud_env=mc*self.mud_env+(1.0-mc)*self.mud_lp.abs();
        let mud_over=((self.mud_env-0.16)/0.52).clamp(0.0,1.0); let mud_red=mud_over*self.cfg.body_control.clamp(0.0,1.0)*(0.14+drive*0.10+competition*0.055);
        x-=self.mud_lp*mud_red; self.last_dynamic_eq_activity=(mud_red/0.24).clamp(0.0,1.0);
        let body_coeff=(-std::f32::consts::TAU*520.0/self.sample_rate).exp(); self.body_lp=body_coeff*self.body_lp+(1.0-body_coeff)*x; x+=self.body_lp*(0.035+density*0.055);

        // Presence with harshness guard.
        let lp_coeff=(-std::f32::consts::TAU*3400.0/self.sample_rate).exp(); self.presence_lp=lp_coeff*self.presence_lp+(1.0-lp_coeff)*x;
        let detail=x-self.presence_lp; let ha=(-1.0/(self.sample_rate*0.0025)).exp(); let hr=(-1.0/(self.sample_rate*0.075)).exp();
        let hc=if detail.abs()>self.harsh_env {ha}else{hr}; self.harsh_env=hc*self.harsh_env+(1.0-hc)*detail.abs();
        let harsh_target=(0.38-competition*0.035).max(0.32); let harsh_guard=if self.harsh_env>harsh_target {(harsh_target/self.harsh_env).clamp(0.46,1.0)}else{1.0};
        x+=detail*(self.cfg.presence_amount+drive*0.25).clamp(0.0,1.55)*harsh_guard;

        // De-esser.
        let sib_coeff=(-std::f32::consts::TAU*5200.0/self.sample_rate).exp(); self.sibilance_lp=sib_coeff*self.sibilance_lp+(1.0-sib_coeff)*x;
        let sib=x-self.sibilance_lp; let sa=(-1.0/(self.sample_rate*0.0012)).exp(); let sr=(-1.0/(self.sample_rate*0.060)).exp();
        let sc=if sib.abs()>self.sibilance_env {sa}else{sr}; self.sibilance_env=sc*self.sibilance_env+(1.0-sc)*sib.abs();
        let sib_over=((self.sibilance_env-0.20)/0.48).clamp(0.0,1.0); let deess_gain=1.0-sib_over*self.cfg.deesser_amount.clamp(0.0,1.0)*0.62;
        x=self.sibilance_lp+sib*deess_gain; self.last_deesser_reduction_db=(-Self::gain_to_db(deess_gain)).max(0.0);

        // Air enhancer after de-essing.
        let air_coeff=(-std::f32::consts::TAU*7500.0/self.sample_rate).exp(); self.air_lp=air_coeff*self.air_lp+(1.0-air_coeff)*x;
        x+=(x-self.air_lp)*self.cfg.air_amount.clamp(0.0,1.0)*(0.12+drive*0.10);

        // Oversampled dual saturation.
        let smart_clean=(1.0-(self.last_limiter_reduction_db/12.0).clamp(0.0,1.0)*(0.12+competition*0.18)).clamp(0.68,1.0); x*=smart_clean;
        let pre_clip=x; let sat_drive=self.cfg.clip_drive+drive*1.45+density*0.35+competition*0.62+overdrive*1.28; let sat2=1.10+drive*0.68+overdrive*0.38;
        x=self.oversampled_saturation(x,sat_drive,sat2)*Self::db_to_gain(drive*3.5+density*0.7+competition*1.2+overdrive*1.8);
        self.last_clip_activity=((pre_clip.abs()-x.abs()).abs()*1.55).clamp(0.0,1.0);
        // Overdrive is intentionally capped before CleanGuard so it adds density rather than uncontrolled digital clipping.
        if overdrive>0.0 { x=Self::soft_clip(x,1.0+overdrive*2.4); }
        x*=Self::db_to_gain(self.cfg.output_gain_db+drive*1.8+competition*0.8);
        self.lookahead_limit(x)
    }
}
