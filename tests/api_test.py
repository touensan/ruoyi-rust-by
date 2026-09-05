"""Integration tests. Refuse to run unless the server explicitly identifies as a disposable test instance.
RUOYI_TEST_MODE=true and CAPTCHA_ENABLED=false are required on that instance.
Credentials come only from the environment; never run this against production.
"""
import base64, io, json, os, unittest, urllib.request, urllib.error, zipfile, uuid, struct, zlib
BASE = os.environ.get('TEST_BASE_URL', 'http://127.0.0.1:18889').rstrip('/')
PASSWORD = os.environ['TEST_ADMIN_PASSWORD']

def request(method, path, body=None, token=None, raw=False, content_type='application/json'):
    headers = {'Content-Type': content_type}
    if token: headers['Authorization'] = 'Bearer ' + token
    payload = None if body is None else (body if isinstance(body, bytes) else json.dumps(body).encode())
    req = urllib.request.Request(BASE + '/api/' + path, payload, headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=20) as r: status, data = r.status, r.read()
    except urllib.error.HTTPError as e: status, data = e.code, e.read()
    return status, data if raw else json.loads(data)

class API(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        health = request('GET','health')[1]
        if health.get('data',{}).get('testMode') is not True:
            raise RuntimeError('Refusing writes: RUOYI_TEST_MODE is not true on target server')
        if request('GET','captchaImage')[1].get('captchaEnabled') is not False:
            raise RuntimeError('Tests require CAPTCHA_ENABLED=false on disposable instance')
        cls.token = request('POST','login',{'username':'admin','password':PASSWORD})[1]['token']
        cls.suffix = uuid.uuid4().hex[:8]
        cls.userpass = 'Test-only-' + uuid.uuid4().hex
        menus = request('GET','system/menu/list',token=cls.token)[1]['data']
        cls.menuids = [r['menuId'] for r in menus if r.get('perms') in ['system:user:list','system:user:query','system:user:edit','system:user:remove','system:user:export','system:role:edit','system:config:list']]
        status,r = request('POST','system/role',{'roleName':'测试本人范围','roleKey':'test_'+cls.suffix,'roleSort':0,'dataScope':'5','menuIds':cls.menuids},cls.token)
        if status != 200: raise RuntimeError(str(r))
        cls.roleid = r['data']
        cls.users = []
        for n in range(2):
            name = 'test'+cls.suffix+str(n)
            status,r = request('POST','system/user',{'userName':name,'nickName':'测试用户'+str(n),'password':cls.userpass,'deptId':103,'roleIds':[cls.roleid],'postIds':[1]},cls.token)
            if status != 200: raise RuntimeError(str(r))
            cls.users.append((r['data'],name))
        cls.member = request('POST','login',{'username':cls.users[0][1],'password':cls.userpass})[1]['token']

    def api(self,method,path,body=None,token=None,expected=200,**kwargs):
        status,result=request(method,path,body,self.token if token is None else token,**kwargs)
        self.assertEqual(status,expected,str(result)[:600]);return result

    def test_authentication_and_serialization(self):
        self.assertEqual(request('GET','getInfo')[0],401)
        info=self.api('GET','getInfo');self.assertEqual(info['permissions'],['*:*:*']);self.assertNotIn('password',info['user'])
        self.assertTrue(self.api('GET','getRouters')['data'])
        users=self.api('GET','system/user/list')['rows'];self.assertTrue(users);self.assertTrue(all('password' not in u for u in users))
        self.assertEqual(request('POST','logout',token='0'*64)[0],200)
        self.assertEqual(request('POST','register',{'username':'anything','password':self.userpass})[0],403)

    def test_unified_backend_role_inheritance(self):
        common=self.api('GET','system/role/2')['data']
        common_menus=self.api('GET','system/menu/roleMenuTreeselect/2')['checkedKeys']
        valid={m['menuId'] for m in self.api('GET','system/menu/list')['data']}
        common_menus=[mid for mid in common_menus if mid in valid]
        created=[]; menu=None
        try:
            menu=self.api('POST','system/menu',{'menuName':'普通功能测试','parentId':0,'path':'member'+self.suffix,'component':'system/user/profile/index','menuType':'C','perms':'system:post:list','status':'0'})['data']
            self.api('PUT','system/role',{'roleId':2,'status':'0','menuIds':[menu]})
            tokens=[]
            for name,roles in [('manager',[1,self.roleid]),('member',[2])]:
                username=name+self.suffix
                created.append(self.api('POST','system/user',{'userName':username,'nickName':'统一后台测试','password':self.userpass,'deptId':103,'roleIds':roles})['data'])
                tokens.append(self.api('POST','login',{'username':username,'password':self.userpass})['token'])
            manager,member=tokens
            info=self.api('GET','getInfo',token=manager)
            self.assertTrue({'admin','common'} <= set(info['roles']))
            self.assertNotIn('common',[r['roleKey'] for r in info['user']['roles']])
            self.assertNotIn('*:*:*',info['permissions']);self.assertFalse(info['user']['admin'])
            self.assertIn('common',self.api('GET','getInfo')['roles'])
            for token in [self.token,manager,member]:
                self.api('GET','system/user/profile',token=token)
                self.api('GET','system/post/list',token=token)
                self.assertIn('member'+self.suffix,json.dumps(self.api('GET','getRouters',token=token)))
            self.api('GET','system/user/list',token=manager)
            self.api('GET','system/user/list',token=member,expected=403)
            self.api('POST','system/role',{'roleName':'forbidden'},manager,403)
            self.api('GET','monitor/server',token=member,expected=403)
            self.api('PUT','system/role',{'roleId':2,'status':'1'})
            for token in [manager,member]:
                self.assertNotIn('common',self.api('GET','getInfo',token=token)['roles'])
                self.api('GET','system/post/list',token=token,expected=403)
                self.assertNotIn('member'+self.suffix,json.dumps(self.api('GET','getRouters',token=token)))
            self.api('PUT','system/role',{'roleId':2,'status':'0'})
            self.api('PUT','system/menu',{'menuId':menu,'status':'1'})
            self.api('GET','system/post/list',token=manager,expected=403)
        finally:
            self.api('PUT','system/role',{'roleId':2,'status':common['status'],'menuIds':common_menus})
            for uid in created:self.api('DELETE','system/user/'+str(uid))
            if menu:self.api('DELETE','system/menu/'+str(menu))

    def test_data_scope_and_privilege_escalation(self):
        rows=self.api('GET','system/user/list',token=self.member)['rows'];self.assertEqual(len(rows),1);self.assertEqual(rows[0]['userId'],self.users[0][0])
        self.api('GET','system/user/'+str(self.users[1][0]),token=self.member,expected=404)
        self.api('PUT','system/user',{'userId':self.users[1][0],'nickName':'forbidden'},self.member,404)
        self.api('PUT','system/user',{'userId':self.users[0][0],'roleIds':[1]},self.member,403)
        self.api('PUT','system/user',{'userId':self.users[0][0],'password':self.userpass},self.member,403)
        self.api('PUT','system/role',{'roleId':self.roleid,'dataScope':'1'},self.member,403)
        self.api('DELETE','system/user/'+str(self.users[1][0]),token=self.member,expected=404)
        csv=self.api('POST','system/user/export',{},self.member,raw=True).decode('utf-8-sig')
        self.assertIn(self.users[0][1],csv);self.assertNotIn(self.users[1][1],csv);self.assertNotIn('password',csv)

    def test_data_scope_variants(self):
        childdept=self.api('POST','system/dept',{'deptName':'范围测试子部门','parentId':103})['data']
        created=[]
        for idx,dept in enumerate([childdept,104]):
            created.append(self.api('POST','system/user',{'userName':'scope'+self.suffix+str(idx),'nickName':'范围测试','password':self.userpass,'deptId':dept})['data'])
        def visible(scope,departments=None):
            body={'roleId':self.roleid,'dataScope':scope}
            if departments is not None:body['deptIds']=departments
            self.api('PUT','system/role/dataScope',body)
            return {r['userId'] for r in self.api('GET','system/user/list?pageSize=100',token=self.member)['rows']}
        same=visible('3');self.assertIn(self.users[1][0],same);self.assertNotIn(created[0],same);self.assertNotIn(created[1],same)
        descendants=visible('4');self.assertIn(created[0],descendants);self.assertNotIn(created[1],descendants)
        custom=visible('2',[104]);self.assertIn(self.users[0][0],custom);self.assertIn(created[1],custom);self.assertNotIn(self.users[1][0],custom)
        allusers=visible('1');self.assertIn(1,allusers);self.assertTrue(set(created)<=allusers)
        self.assertEqual(visible('5'),{self.users[0][0]})
        for uid in created:self.api('DELETE','system/user/'+str(uid))
        self.api('DELETE','system/dept/'+str(childdept))

    def test_dictionary_config_crud_and_filters(self):
        label='test.dict.'+self.suffix
        d=self.api('POST','system/dict/type',{'dictName':'测试字典','dictType':label})['data']
        value=self.api('POST','system/dict/data',{'dictLabel':'测试项','dictValue':'yes','dictType':label})['data']
        self.assertEqual(self.api('GET','system/dict/data/type/'+label)['data'][0]['dictValue'],'yes')
        self.api('DELETE','system/dict/type/'+str(d),expected=422)
        self.api('DELETE','system/dict/data/'+str(value));self.api('DELETE','system/dict/type/'+str(d))
        c=self.api('POST','system/config',{'configName':'测试配置','configKey':'test.'+self.suffix,'configValue':'=formula'})['data']
        self.api('PUT','system/config',{'configId':c,'configValue':'changed'})
        self.assertEqual(self.api('GET','system/config/configKey/test.'+self.suffix)['msg'],'changed')
        self.api('DELETE','system/config/'+str(c))
        self.assertEqual(self.api('GET','system/user/list?userName=%27%20OR%201%3D1--')['rows'],[])
        self.api('DELETE','system/user/1,2%20OR%201=1',expected=422)

    def test_department_cycle_and_relations(self):
        parent=self.api('POST','system/dept',{'deptName':'测试父部门','parentId':100})['data']
        child=self.api('POST','system/dept',{'deptName':'测试子部门','parentId':parent})['data']
        self.api('PUT','system/dept',{'deptId':parent,'parentId':child},expected=422)
        self.api('DELETE','system/dept/'+str(parent),expected=422)
        self.api('DELETE','system/dept/'+str(child));self.api('DELETE','system/dept/'+str(parent))
        detail=self.api('GET','system/user/'+str(self.users[0][0]));self.assertEqual(detail['roleIds'],[self.roleid]);self.assertEqual(detail['postIds'],[1])
        self.api('PUT','system/user',{'userId':self.users[0][0],'roleIds':[999999999]},expected=422)
        self.api('DELETE','system/user/1',expected=403)

    def test_settings_secret_redaction_and_unsupported_operations(self):
        value=self.api('GET','system/setting')['data'];self.assertEqual(value['site']['title'],'RuoYi-Rust BY')
        self.api('PUT','system/setting/mail',{'password':'private-test-value','port':465,'encryption':'ssl'})
        self.assertEqual(self.api('GET','system/setting')['data']['mail']['password'],'********')
        self.api('PUT','system/setting/mail',{'password':'********','fromName':'测试'})
        self.assertEqual(self.api('GET','system/setting')['data']['mail']['password'],'********')
        self.api('PUT','system/setting/payment',{'enabled':True},expected=422)
        self.api('POST','system/setting/mail/test',{},expected=422)
        self.api('PUT','system/setting/site',{'title':'RuoYi-Rust BY','frontendHeadCode':'<script>alert(1)</script>'})
        self.assertEqual(request('GET','site/config')[1]['data']['frontendHeadCode'],'')
        self.api('GET','system/setting',token=self.member,expected=403)

    def test_generator_metadata_zip_and_secrets(self):
        self.api('POST','tool/gen/importTable?tables=sys_user')
        rows=self.api('GET','tool/gen/list?tableName=sys_user')['rows'];gid=rows[0]['tableId']
        files=self.api('GET','tool/gen/preview/'+str(gid))['data'];self.assertIn('model.rs',files);self.assertNotIn('password',files['model.rs']);self.assertIn('query_as',files['service.rs'])
        archive=self.api('GET','tool/gen/batchGenCode?tables=sys_user',raw=True)
        with zipfile.ZipFile(io.BytesIO(archive)) as z:self.assertIn('sys_user/model.rs',z.namelist())
        self.api('POST','tool/gen/createTable?sql=DROP%20TABLE%20sys_user',expected=422)
        self.api('GET','tool/gen/list',token=self.member,expected=403)
        self.api('DELETE','tool/gen/'+str(gid))

    def test_notices_and_audit(self):
        nid=self.api('POST','system/notice',{'noticeTitle':'集成测试公告','noticeContent':'<p>测试内容</p>','noticeType':'1'})['data']
        top=self.api('GET','system/notice/listTop');self.assertTrue(any(r['noticeId']==nid for r in top['data']))
        self.api('POST','system/notice/markRead',{'noticeId':nid})
        readers=self.api('GET','system/notice/readUsers/list?noticeId='+str(nid));self.assertEqual(readers['total'],1)
        self.api('DELETE','system/notice/'+str(nid))
        logs=self.api('GET','monitor/operlog/list')['rows'];self.assertTrue(logs)
        self.assertNotIn(PASSWORD,json.dumps(logs));self.assertNotIn(self.token,json.dumps(logs))
        self.assertTrue(self.api('GET','monitor/server')['data']['cpu']['cpuNum'])

    def test_upload_rejects_active_content(self):
        boundary='----ruoyibytest'
        payload=(f'--{boundary}\r\nContent-Disposition: form-data; name="file"; filename="evil.svg"\r\nContent-Type: image/svg+xml\r\n\r\n<svg onload="alert(1)"></svg>\r\n--{boundary}--\r\n').encode()
        self.api('POST','common/upload',payload,expected=422,content_type='multipart/form-data; boundary='+boundary)
        self.api('POST','common/upload',payload,token=self.member,expected=403,content_type='multipart/form-data; boundary='+boundary)
        def chunk(kind,body):return struct.pack('!I',len(body))+kind+body+struct.pack('!I',zlib.crc32(kind+body)&0xffffffff)
        png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('!IIBBBBB',1,1,8,2,0,0,0))+chunk(b'IDAT',zlib.compress(b'\x00\x20\x60\xa0'))+chunk(b'IEND',b'')
        payload=(f'--{boundary}\r\nContent-Disposition: form-data; name="file"; filename="test.png"\r\nContent-Type: image/png\r\n\r\n').encode()+png+(f'\r\n--{boundary}--\r\n').encode()
        uploaded=self.api('POST','common/upload',payload,content_type='multipart/form-data; boundary='+boundary)
        self.assertTrue(uploaded['url'].startswith(BASE+'/uploads/'))
        with urllib.request.urlopen(uploaded['url']) as r:self.assertTrue(r.read().startswith(b'\x89PNG\r\n\x1a\n'))


    def test_z_password_change_and_revocation(self):
        uid,name=self.users[1];old=request('POST','login',{'username':name,'password':self.userpass})[1]['token']
        self.api('PUT','system/user/profile/updatePwd',{'oldPassword':self.userpass,'newPassword':'short'},old,422)
        newpass='New-'+uuid.uuid4().hex
        self.api('PUT','system/user/profile/updatePwd',{'oldPassword':self.userpass,'newPassword':newpass},old)
        self.assertEqual(request('GET','getInfo',token=old)[0],401)
        self.assertEqual(request('POST','logout',token=old)[0],200)
        fresh=request('POST','login',{'username':name,'password':newpass})[1]['token']
        self.api('PUT','system/user/changeStatus',{'userId':uid,'status':'1'})
        self.assertEqual(request('GET','getInfo',token=fresh)[0],401)

if __name__ == '__main__': unittest.main(verbosity=2)
