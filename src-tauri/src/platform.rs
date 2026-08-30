use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{env, fs, path::PathBuf, sync::Mutex, time::{SystemTime, UNIX_EPOCH}};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const PASSWORD_SALT: &str = "flaw-loud-connected-platform-v11";
const MAX_ATTACHMENTS: usize = 3;
const MAX_ATTACHMENT_BYTES: usize = 25 * 1024 * 1024;

fn now() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) }
fn data_dir() -> PathBuf {
    let base = env::var("LOCALAPPDATA").unwrap_or_else(|_| env::temp_dir().to_string_lossy().to_string());
    let dir = PathBuf::from(base).join("Flaw Loud").join("ConnectedPlatform");
    let _ = fs::create_dir_all(&dir);
    dir
}
fn db_path() -> PathBuf { data_dir().join("platform.json") }
fn attachment_dir() -> PathBuf { let p=data_dir().join("attachments"); let _=fs::create_dir_all(&p); p }
fn hash_password(username: &str, password: &str) -> String {
    format!("{:x}", Sha256::digest(format!("{}|{}|{}", PASSWORD_SALT, username.to_lowercase(), password).as_bytes()))
}
fn session_token(user_id: u64) -> String {
    format!("{:x}", Sha256::digest(format!("session|{}|{}|{}", user_id, now(), std::process::id()).as_bytes()))
}
fn sanitize_filename(v: &str) -> String {
    let mut s=v.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c,'.'|'_'|'-') {c} else {'_'}).collect::<String>();
    if s.len()>100 { s.truncate(100); }
    if s.is_empty(){"attachment.bin".into()}else{s}
}
fn parse_version(v: &str) -> Vec<u64> {
    let clean=v.trim_start_matches('v').split('-').next().unwrap_or(v);
    clean.split('.').map(|p| p.parse::<u64>().unwrap_or(0)).collect()
}
fn version_lt(a:&str,b:&str)->bool{
    let mut av=parse_version(a); let mut bv=parse_version(b); let n=av.len().max(bv.len()); av.resize(n,0);bv.resize(n,0);av<bv
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PlatformUser {
    pub id:u64,
    pub username:String,
    pub password_hash:String,
    pub role:String,
    pub status:String,
    pub suspended_until:u64,
    pub created_at:u64,
    pub last_login:u64,
}
#[derive(Clone, Serialize)]
pub struct PublicUser {
    pub id:u64,pub username:String,pub role:String,pub status:String,pub suspended_until:u64,pub created_at:u64,pub last_login:u64
}
impl From<&PlatformUser> for PublicUser { fn from(u:&PlatformUser)->Self{Self{id:u.id,username:u.username.clone(),role:u.role.clone(),status:u.status.clone(),suspended_until:u.suspended_until,created_at:u.created_at,last_login:u.last_login}} }
#[derive(Clone, Serialize, Deserialize)]
pub struct PlatformSession { pub token:String, pub user_id:u64, pub created_at:u64 }
#[derive(Clone, Serialize, Deserialize)]
pub struct Announcement {
    pub id:u64,pub title:String,pub message:String,pub priority:String,pub version:String,pub created_by:String,pub created_at:u64
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Report {
    pub id:u64,pub user_id:u64,pub username:String,pub category:String,pub message:String,pub attachments:Vec<String>,pub status:String,pub created_at:u64,pub updated_at:u64
}
#[derive(Clone, Serialize, Deserialize)]
pub struct AuditEvent { pub id:u64,pub actor:String,pub action:String,pub target:String,pub detail:String,pub created_at:u64 }
#[derive(Clone, Serialize, Deserialize)]
pub struct ReleasePolicy {
    pub latest_version:String,
    pub minimum_supported_version:String,
    pub grace_until:u64,
    pub notes:String,
    pub force_after_grace:bool,
}
impl Default for ReleasePolicy {
    fn default()->Self{Self{latest_version:APP_VERSION.into(),minimum_supported_version:"0.0.0".into(),grace_until:0,notes:"Flaw Loud v1.1 Connected Platform RC".into(),force_after_grace:true}}
}
#[derive(Clone, Serialize, Deserialize)]
pub struct PlatformDb {
    pub users:Vec<PlatformUser>,pub sessions:Vec<PlatformSession>,pub announcements:Vec<Announcement>,pub reads:Vec<(u64,u64)>,pub reports:Vec<Report>,pub audit:Vec<AuditEvent>,pub release_policy:ReleasePolicy,pub next_id:u64
}
impl Default for PlatformDb { fn default()->Self{Self{users:vec![],sessions:vec![],announcements:vec![],reads:vec![],reports:vec![],audit:vec![],release_policy:ReleasePolicy::default(),next_id:1}} }

pub struct PlatformState { db:Mutex<PlatformDb> }
impl Default for PlatformState {
    fn default()->Self{
        let db=fs::read_to_string(db_path()).ok().and_then(|s|serde_json::from_str::<PlatformDb>(&s).ok()).unwrap_or_default();
        Self{db:Mutex::new(db)}
    }
}
impl PlatformState {
    fn save(db:&PlatformDb)->Result<(),String>{
        fs::write(db_path(),serde_json::to_vec_pretty(db).map_err(|e|e.to_string())?).map_err(|e|e.to_string())
    }
}

#[derive(Clone, Serialize)]
pub struct BootstrapStatus { pub has_owner:bool,pub local_backend:bool,pub app_version:String }
#[derive(Clone, Serialize)]
pub struct SessionView {
    pub authenticated:bool,pub token:String,pub username:String,pub role:String,pub user_id:u64,
    pub update_available:bool,pub update_required:bool,pub owner_bypass:bool,pub latest_version:String,pub grace_until:u64,pub update_notes:String,pub message:String
}
#[derive(Clone, Serialize)]
pub struct AnnouncementFeed { pub unread_count:usize,pub unread_ids:Vec<u64>,pub items:Vec<Announcement> }
#[derive(Clone, Serialize)]
pub struct AdminSnapshot { pub users:Vec<PublicUser>,pub reports:Vec<Report>,pub audit:Vec<AuditEvent>,pub release_policy:ReleasePolicy }
#[derive(Clone, Deserialize)]
pub struct AttachmentInput { pub name:String,pub mime:String,pub data_base64:String }

fn get_user_for_token<'a>(db:&'a PlatformDb,token:&str)->Result<&'a PlatformUser,String>{
    let sess=db.sessions.iter().find(|s|s.token==token).ok_or_else(||"Session expired. Please sign in again.".to_string())?;
    db.users.iter().find(|u|u.id==sess.user_id).ok_or_else(||"Account no longer exists.".to_string())
}
fn session_view(db:&PlatformDb,token:String,current_version:&str)->Result<SessionView,String>{
    let user=get_user_for_token(db,&token)?.clone(); let t=now();
    if user.status=="banned" { return Err("This account is banned.".into()); }
    if user.status=="blocked" { return Err("Sign-in access is blocked for this account.".into()); }
    if user.suspended_until>t { return Err(format!("Account suspended until {}.",user.suspended_until)); }
    let owner=user.role=="Owner";
    let p=&db.release_policy;
    let below_floor=version_lt(current_version,&p.minimum_supported_version);
    let behind=version_lt(current_version,&p.latest_version);
    let after_grace=p.force_after_grace && p.grace_until>0 && t>=p.grace_until;
    let required=!owner && (below_floor || (behind && after_grace));
    Ok(SessionView{authenticated:true,token,username:user.username,role:user.role,user_id:user.id,update_available:behind,update_required:required,owner_bypass:owner&&behind,latest_version:p.latest_version.clone(),grace_until:p.grace_until,update_notes:p.notes.clone(),message:if required{"Update required before the engine can start.".into()}else if behind{"A newer Flaw Loud version is available.".into()}else{"Connected Platform ready.".into()}})
}
fn next_id(db:&mut PlatformDb)->u64{let id=db.next_id;db.next_id+=1;id}
fn audit(db:&mut PlatformDb,actor:String,action:&str,target:String,detail:String){let id=next_id(db);db.audit.insert(0,AuditEvent{id,actor,action:action.into(),target,detail,created_at:now()});if db.audit.len()>500{db.audit.truncate(500)}}
fn require_admin(db:&PlatformDb,token:&str)->Result<PlatformUser,String>{let u=get_user_for_token(db,token)?.clone();if u.role=="Owner"||u.role=="Moderator"{Ok(u)}else{Err("Moderator or Owner access required.".into())}}
fn require_owner(db:&PlatformDb,token:&str)->Result<PlatformUser,String>{let u=get_user_for_token(db,token)?.clone();if u.role=="Owner"{Ok(u)}else{Err("Owner access required.".into())}}

#[tauri::command]
pub fn platform_bootstrap_status(state:tauri::State<'_,PlatformState>)->Result<BootstrapStatus,String>{
    let db=state.db.lock().map_err(|_|"Platform state lock failed")?;
    Ok(BootstrapStatus{has_owner:db.users.iter().any(|u|u.role=="Owner"),local_backend:true,app_version:APP_VERSION.into()})
}
#[tauri::command]
pub fn platform_bootstrap_owner(username:String,password:String,current_version:String,state:tauri::State<'_,PlatformState>)->Result<SessionView,String>{
    let mut db=state.db.lock().map_err(|_|"Platform state lock failed")?;
    if db.users.iter().any(|u|u.role=="Owner"){return Err("Owner account already exists.".into())}
    if username.trim().len()<3 || password.len()<8{return Err("Use at least 3 characters for username and 8 for password.".into())}
    let id=next_id(&mut db);let t=now();let clean=username.trim().to_string();
    db.users.push(PlatformUser{id,username:clean.clone(),password_hash:hash_password(&clean,&password),role:"Owner".into(),status:"active".into(),suspended_until:0,created_at:t,last_login:t});
    let token=session_token(id);db.sessions.push(PlatformSession{token:token.clone(),user_id:id,created_at:t});
    audit(&mut db,clean.clone(),"BOOTSTRAP_OWNER",clean.clone(),"Initial local Connected Platform owner created".into());
    PlatformState::save(&db)?;session_view(&db,token,&current_version)
}
#[tauri::command]
pub fn platform_login(username:String,password:String,current_version:String,state:tauri::State<'_,PlatformState>)->Result<SessionView,String>{
    let mut db=state.db.lock().map_err(|_|"Platform state lock failed")?;let t=now();
    let idx=db.users.iter().position(|u|u.username.eq_ignore_ascii_case(username.trim())).ok_or_else(||"Invalid username or password.".to_string())?;
    let expected=hash_password(&db.users[idx].username,&password);if expected!=db.users[idx].password_hash{return Err("Invalid username or password.".into())}
    if db.users[idx].status=="banned"{return Err("This account is banned.".into())}if db.users[idx].status=="blocked"{return Err("Sign-in access is blocked for this account.".into())}if db.users[idx].suspended_until>t{return Err("This account is temporarily suspended.".into())}
    db.users[idx].last_login=t;let uid=db.users[idx].id;let actor=db.users[idx].username.clone();db.sessions.retain(|s|s.user_id!=uid);let token=session_token(uid);db.sessions.push(PlatformSession{token:token.clone(),user_id:uid,created_at:t});audit(&mut db,actor.clone(),"LOGIN",actor,"Connected Platform sign-in".into());PlatformState::save(&db)?;session_view(&db,token,&current_version)
}
#[tauri::command]
pub fn platform_resume_session(token:String,current_version:String,state:tauri::State<'_,PlatformState>)->Result<SessionView,String>{let db=state.db.lock().map_err(|_|"Platform state lock failed")?;session_view(&db,token,&current_version)}
#[tauri::command]
pub fn platform_logout(token:String,state:tauri::State<'_,PlatformState>)->Result<(),String>{let mut db=state.db.lock().map_err(|_|"Platform state lock failed")?;db.sessions.retain(|s|s.token!=token);PlatformState::save(&db)}

#[tauri::command]
pub fn platform_list_announcements(token:String,state:tauri::State<'_,PlatformState>)->Result<AnnouncementFeed,String>{let db=state.db.lock().map_err(|_|"Platform state lock failed")?;let u=get_user_for_token(&db,&token)?;let mut items=db.announcements.clone();items.sort_by_key(|a|std::cmp::Reverse(a.created_at));let unread_ids=items.iter().filter(|a|!db.reads.contains(&(u.id,a.id))).map(|a|a.id).collect::<Vec<_>>();let unread_count=unread_ids.len();Ok(AnnouncementFeed{unread_count,unread_ids,items})}
#[tauri::command]
pub fn platform_mark_announcement_read(token:String,announcement_id:u64,state:tauri::State<'_,PlatformState>)->Result<(),String>{let mut db=state.db.lock().map_err(|_|"Platform state lock failed")?;let uid=get_user_for_token(&db,&token)?.id;if !db.reads.contains(&(uid,announcement_id)){db.reads.push((uid,announcement_id));}PlatformState::save(&db)}
#[tauri::command]
pub fn platform_create_announcement(token:String,title:String,message:String,priority:String,version:String,state:tauri::State<'_,PlatformState>)->Result<Announcement,String>{let mut db=state.db.lock().map_err(|_|"Platform state lock failed")?;let actor=require_admin(&db,&token)?;if title.trim().is_empty()||message.trim().is_empty(){return Err("Title and message are required.".into())}let a=Announcement{id:next_id(&mut db),title:title.trim().into(),message:message.trim().into(),priority:priority.trim().into(),version:version.trim().into(),created_by:actor.username.clone(),created_at:now()};db.announcements.push(a.clone());audit(&mut db,actor.username,"ANNOUNCEMENT",a.title.clone(),format!("Priority {} · version {}",a.priority,a.version));PlatformState::save(&db)?;Ok(a)}

#[tauri::command]
pub fn platform_submit_report(token:String,category:String,message:String,attachments:Vec<AttachmentInput>,state:tauri::State<'_,PlatformState>)->Result<Report,String>{
    let mut db=state.db.lock().map_err(|_|"Platform state lock failed")?;let u=get_user_for_token(&db,&token)?.clone();if message.trim().len()<8{return Err("Please describe the problem with a little more detail.".into())}if attachments.len()>MAX_ATTACHMENTS{return Err("Maximum 3 attachments per report.".into())}
    let report_id=next_id(&mut db);let mut stored=Vec::new();
    for (i,a) in attachments.iter().enumerate(){let raw=B64.decode(a.data_base64.as_bytes()).map_err(|_|"Invalid attachment data.".to_string())?;if raw.len()>MAX_ATTACHMENT_BYTES{return Err(format!("{} is larger than 25 MB.",a.name))}let safe=sanitize_filename(&a.name);let path=attachment_dir().join(format!("report_{}_{}_{}",report_id,i,safe));fs::write(&path,&raw).map_err(|e|format!("Could not save attachment: {e}"))?;stored.push(path.to_string_lossy().to_string());let _=&a.mime;}
    let t=now();let r=Report{id:report_id,user_id:u.id,username:u.username.clone(),category:category.trim().into(),message:message.trim().into(),attachments:stored,status:"New".into(),created_at:t,updated_at:t};db.reports.push(r.clone());audit(&mut db,u.username,"REPORT_SUBMITTED",format!("Report #{}",r.id),r.category.clone());PlatformState::save(&db)?;Ok(r)
}

#[tauri::command]
pub fn platform_admin_snapshot(token:String,state:tauri::State<'_,PlatformState>)->Result<AdminSnapshot,String>{let db=state.db.lock().map_err(|_|"Platform state lock failed")?;require_admin(&db,&token)?;let mut users=db.users.iter().map(PublicUser::from).collect::<Vec<_>>();users.sort_by(|a,b|a.username.to_lowercase().cmp(&b.username.to_lowercase()));let mut reports=db.reports.clone();reports.sort_by_key(|r|std::cmp::Reverse(r.updated_at));Ok(AdminSnapshot{users,reports,audit:db.audit.clone(),release_policy:db.release_policy.clone()})}
#[tauri::command]
pub fn platform_create_user(token:String,username:String,password:String,role:String,state:tauri::State<'_,PlatformState>)->Result<PublicUser,String>{let mut db=state.db.lock().map_err(|_|"Platform state lock failed")?;let actor=require_owner(&db,&token)?;let clean=username.trim();if clean.len()<3||password.len()<8{return Err("Username must be 3+ chars and password 8+ chars.".into())}if db.users.iter().any(|u|u.username.eq_ignore_ascii_case(clean)){return Err("Username already exists.".into())}let role=if role=="Moderator"{"Moderator"}else{"User"};let u=PlatformUser{id:next_id(&mut db),username:clean.into(),password_hash:hash_password(clean,&password),role:role.into(),status:"active".into(),suspended_until:0,created_at:now(),last_login:0};db.users.push(u.clone());audit(&mut db,actor.username,"CREATE_USER",u.username.clone(),format!("Role {}",u.role));PlatformState::save(&db)?;Ok(PublicUser::from(&u))}
#[tauri::command]
pub fn platform_set_user_role(token:String,user_id:u64,role:String,state:tauri::State<'_,PlatformState>)->Result<(),String>{let mut db=state.db.lock().map_err(|_|"Platform state lock failed")?;let actor=require_owner(&db,&token)?;let idx=db.users.iter().position(|u|u.id==user_id).ok_or_else(||"User not found.".to_string())?;if db.users[idx].role=="Owner"{return Err("The Owner role cannot be modified here.".into())}let new_role=if role=="Moderator"{"Moderator"}else{"User"};let target=db.users[idx].username.clone();db.users[idx].role=new_role.into();db.sessions.retain(|s|s.user_id!=user_id);audit(&mut db,actor.username,"ROLE_CHANGED",target,format!("New role {}",new_role));PlatformState::save(&db)}
#[tauri::command]
pub fn platform_set_user_access(token:String,user_id:u64,status:String,suspend_hours:u64,state:tauri::State<'_,PlatformState>)->Result<(),String>{let mut db=state.db.lock().map_err(|_|"Platform state lock failed")?;let actor=require_owner(&db,&token)?;let idx=db.users.iter().position(|u|u.id==user_id).ok_or_else(||"User not found.".to_string())?;if db.users[idx].role=="Owner"{return Err("Owner access cannot be blocked.".into())}let target=db.users[idx].username.clone();match status.as_str(){"active"=>{db.users[idx].status="active".into();db.users[idx].suspended_until=0},"suspended"=>{db.users[idx].status="active".into();db.users[idx].suspended_until=now()+suspend_hours.max(1)*3600},"blocked"=>{db.users[idx].status="blocked".into();db.users[idx].suspended_until=0},"banned"=>{db.users[idx].status="banned".into();db.users[idx].suspended_until=0},_=>return Err("Unknown access status.".into())}db.sessions.retain(|s|s.user_id!=user_id);audit(&mut db,actor.username,"ACCESS_CHANGED",target,format!("{}{}",status,if status=="suspended"{format!(" for {}h",suspend_hours)}else{String::new()}));PlatformState::save(&db)}
#[tauri::command]
pub fn platform_revoke_sessions(token:String,user_id:u64,state:tauri::State<'_,PlatformState>)->Result<(),String>{let mut db=state.db.lock().map_err(|_|"Platform state lock failed")?;let actor=require_owner(&db,&token)?;let target=db.users.iter().find(|u|u.id==user_id).map(|u|u.username.clone()).ok_or_else(||"User not found.".to_string())?;db.sessions.retain(|s|s.user_id!=user_id);audit(&mut db,actor.username,"SESSIONS_REVOKED",target,"All active sessions closed".into());PlatformState::save(&db)}
#[tauri::command]
pub fn platform_update_report(token:String,report_id:u64,status:String,state:tauri::State<'_,PlatformState>)->Result<(),String>{let mut db=state.db.lock().map_err(|_|"Platform state lock failed")?;let actor=require_admin(&db,&token)?;let idx=db.reports.iter().position(|r|r.id==report_id).ok_or_else(||"Report not found.".to_string())?;let allowed=["New","Reviewing","Fixed","Closed"];if !allowed.contains(&status.as_str()){return Err("Invalid report status.".into())}db.reports[idx].status=status.clone();db.reports[idx].updated_at=now();audit(&mut db,actor.username,"REPORT_STATUS",format!("Report #{}",report_id),status);PlatformState::save(&db)}
#[tauri::command]
pub fn platform_set_release_policy(token:String,latest_version:String,minimum_supported_version:String,notes:String,grace_hours:u64,state:tauri::State<'_,PlatformState>)->Result<ReleasePolicy,String>{let mut db=state.db.lock().map_err(|_|"Platform state lock failed")?;let actor=require_owner(&db,&token)?;if latest_version.trim().is_empty(){return Err("Latest version is required.".into())}let p=ReleasePolicy{latest_version:latest_version.trim().trim_start_matches('v').into(),minimum_supported_version:if minimum_supported_version.trim().is_empty(){"0.0.0".into()}else{minimum_supported_version.trim().trim_start_matches('v').into()},grace_until:now()+grace_hours.max(1)*3600,notes:notes.trim().into(),force_after_grace:true};db.release_policy=p.clone();audit(&mut db,actor.username,"RELEASE_POLICY",p.latest_version.clone(),format!("{}h grace · minimum {}",grace_hours.max(1),p.minimum_supported_version));PlatformState::save(&db)?;Ok(p)}
#[tauri::command]
pub fn platform_open_attachment(token:String,path:String,state:tauri::State<'_,PlatformState>)->Result<(),String>{
    let db=state.db.lock().map_err(|_|"Platform state lock failed")?;require_admin(&db,&token)?;drop(db);
    let requested=PathBuf::from(&path);let root=attachment_dir().canonicalize().map_err(|e|e.to_string())?;let resolved=requested.canonicalize().map_err(|e|e.to_string())?;
    if !resolved.starts_with(&root){return Err("Attachment path is outside the report storage folder.".into())}
    #[cfg(target_os="windows")] { std::process::Command::new("explorer.exe").arg("/select,").arg(&resolved).spawn().map_err(|e|e.to_string())?; }
    #[cfg(not(target_os="windows"))] { return Err("Attachment reveal is currently implemented for Windows builds.".into()); }
    Ok(())
}
