import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import request from 'supertest';
import { createApp, bootstrapAdmin } from '../src/app.js';

class MemoryStore {
  users=[]; sessions=[]; next=1;
  async createUser({username,passwordHash,role='user'}) { if(this.users.some(u=>u.username.toLowerCase()===username.toLowerCase())) { const e=new Error(); e.code='23505'; throw e; } const u={id:this.next++,username,password_hash:passwordHash,role,created_at:new Date(),last_login_at:null,last_seen_at:null};this.users.push(u);return u; }
  async findUserByUsername(n){return this.users.find(u=>u.username.toLowerCase()===n.toLowerCase())??null}
  async createSession({userId,tokenHash,expiresAt}){this.sessions.push({session_id:this.sessions.length+1,userId,tokenHash,expiresAt})}
  async getSession(h){const s=this.sessions.find(x=>x.tokenHash===h&&x.expiresAt>Date.now());const u=s&&this.users.find(x=>x.id===s.userId);return u?{...u,...s}:null}
  async touchSession(_sid,id){const u=this.users.find(x=>x.id===id);u.last_seen_at=new Date()}
  async markLogin(id){const u=this.users.find(x=>x.id===id);u.last_login_at=new Date();u.last_seen_at=new Date()}
  async deleteSession(h){this.sessions=this.sessions.filter(x=>x.tokenHash!==h)}
  async listUsers(){return this.users.map(u=>({...u,status:'online'}))}
  async getUser(id){return this.users.find(u=>String(u.id)===String(id))??null}
  async stats(){return{totalUsers:this.users.length,admins:this.users.filter(u=>u.role==='admin').length,onlineUsers:this.users.length}}
}

const frontendDir=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../../dist');
const env={NODE_ENV:'test',FLAW_ADMIN_USERNAME:'rootadmin',FLAW_ADMIN_PASSWORD:'Correct-Horse-123'};

test('required routing, auth, and role behavior',async()=>{
 const store=new MemoryStore(); await bootstrapAdmin(store,env); const app=createApp({store,env,frontendDir});
 let r=await request(app).get('/api/health'); assert.equal(r.status,200);assert.match(r.headers['content-type'],/application\/json/);assert.equal(r.body.service,'flaw-loud-api');
 r=await request(app).get('/api/does-not-exist');assert.equal(r.status,404);assert.match(r.headers['content-type'],/application\/json/);assert.deepEqual(r.body,{error:'API route not found'});
 r=await request(app).get('/');assert.equal(r.status,200);assert.match(r.headers['content-type'],/text\/html/);assert.match(r.text,/Flaw Loud/i);
 const user=request.agent(app);
 r=await user.post('/api/auth/register').send({username:'normaluser',password:'Secure-pass-123'});assert.equal(r.status,201);assert.equal(r.body.user.role,'user');assert.equal('password_hash' in r.body.user,false);
 r=await user.post('/api/auth/logout');assert.equal(r.status,204);
 r=await user.post('/api/auth/login').send({username:'normaluser',password:'Secure-pass-123'});assert.equal(r.status,200);
 r=await user.get('/api/auth/me');assert.equal(r.status,200);assert.equal(r.body.user.username,'normaluser');
 r=await user.get('/api/admin/users');assert.equal(r.status,403);
 const admin=request.agent(app);
 r=await admin.post('/api/auth/login').send({username:'rootadmin',password:'Correct-Horse-123',rememberMe:true});assert.equal(r.status,200);assert.equal(r.body.user.role,'admin');
 r=await admin.get('/api/admin/users');assert.equal(r.status,200);assert.equal(r.body.users.length,2);assert.equal(r.body.users.some(x=>'password_hash' in x),false);
 r=await admin.get('/api/admin/stats');assert.equal(r.status,200);assert.equal(r.body.totalUsers,2);
});

test('Tauri origin supports credentialed auth with production cookies',async()=>{
 const store=new MemoryStore();const productionEnv={NODE_ENV:'production',FLAW_ADMIN_USERNAME:'Bnet',FLAW_ADMIN_PASSWORD:'Admin-pass-123'};await bootstrapAdmin(store,productionEnv);const app=createApp({store,env:productionEnv,frontendDir});
 const origin='tauri://localhost';
 let r=await request(app).options('/api/auth/login').set('Origin',origin).set('Access-Control-Request-Method','POST').set('Access-Control-Request-Headers','content-type');
 assert.equal(r.status,204);assert.equal(r.headers['access-control-allow-origin'],origin);assert.equal(r.headers['access-control-allow-credentials'],'true');assert.match(r.headers['access-control-allow-methods'],/GET/);assert.match(r.headers['access-control-allow-methods'],/POST/);assert.match(r.headers['access-control-allow-methods'],/OPTIONS/);assert.match(r.headers['access-control-allow-headers'],/Content-Type/i);

 r=await request(app).post('/api/auth/register').set('Origin',origin).send({username:'tauriuser',password:'Secure-pass-123'});
 assert.equal(r.status,201);assert.equal(r.headers['access-control-allow-origin'],origin);const setCookie=r.headers['set-cookie'][0];assert.match(setCookie,/HttpOnly/i);assert.match(setCookie,/Secure/i);assert.match(setCookie,/SameSite=None/i);const cookie=setCookie.split(';')[0];
 r=await request(app).get('/api/auth/me').set('Origin',origin).set('Cookie',cookie);assert.equal(r.status,200);assert.equal(r.body.user.username,'tauriuser');
 r=await request(app).get('/api/admin/stats').set('Origin',origin).set('Cookie',cookie);assert.equal(r.status,403);
 r=await request(app).post('/api/auth/logout').set('Origin',origin).set('Cookie',cookie);assert.equal(r.status,204);assert.match(r.headers['set-cookie'][0],/SameSite=None/i);
 r=await request(app).post('/api/auth/login').set('Origin',origin).send({username:'tauriuser',password:'Secure-pass-123',rememberMe:true});assert.equal(r.status,200);assert.match(r.headers['set-cookie'][0],/SameSite=None/i);

 r=await request(app).post('/api/auth/login').set('Origin',origin).send({username:'Bnet',password:'Admin-pass-123'});assert.equal(r.status,200);const adminCookie=r.headers['set-cookie'][0].split(';')[0];
 r=await request(app).get('/api/admin/stats').set('Origin',origin).set('Cookie',adminCookie);assert.equal(r.status,200);assert.equal(r.body.admins,1);

 r=await request(app).options('/api/auth/login').set('Origin','https://untrusted.example').set('Access-Control-Request-Method','POST');assert.equal(r.headers['access-control-allow-origin'],undefined);
});
