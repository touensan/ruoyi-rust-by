"""Real HTTP/MySQL/Redis tests with loopback-only Epay and SMTP protocol fixtures.
No real merchant or mailbox is contacted. Requires the disposable test-mode API.
Run with the same TEST_* env as api_test.py and TEST_REDIS_PORT (default 16391).
"""
import api_test as base
import base64, hashlib, http.server, io, json, os, pathlib, socket, socketserver
import subprocess, tempfile, threading, time, unittest, urllib.parse, uuid, zipfile
from xml.sax.saxutils import escape

def canonical(p):
    return '&'.join(k+'='+str(p[k]) for k in sorted(p) if k not in ('sign','sign_type') and str(p[k]).strip())

class Gateway(http.server.BaseHTTPRequestHandler):
    orders = {}
    keydir = None
    merchant = 'test-merchant'
    v1key = 'test-gateway-key'
    tamper = False
    def log_message(self, *args): pass
    @classmethod
    def signed(cls, params, v2=False):
        p = dict(params)
        if v2:
            p['timestamp'] = str(int(time.time()))
            signature = subprocess.run(['openssl','dgst','-sha256','-sign',str(cls.keydir/'platform.pem')],input=canonical(p).encode(),check=True,capture_output=True).stdout
            p.update(sign=base64.b64encode(signature).decode(),sign_type='RSA')
        else:
            p.update(sign=hashlib.md5((canonical(p)+cls.v1key).encode()).hexdigest(),sign_type='MD5')
        return p
    def do_POST(self):
        p = dict(urllib.parse.parse_qsl(self.rfile.read(int(self.headers['Content-Length'])).decode()))
        v2 = self.path.startswith('/api/')
        valid = p.get('pid') == self.merchant
        if v2:
            signature=self.keydir/'request.sig';signature.write_bytes(base64.b64decode(p.get('sign','')))
            valid &= subprocess.run(['openssl','dgst','-sha256','-verify',str(self.keydir/'merchant.pub'),'-signature',str(signature)],input=canonical(p).encode(),capture_output=True).returncode == 0
        elif self.path == '/mapi.php':
            valid &= self.signed(p)['sign'] == p.get('sign')
        else: valid &= p.get('key') == self.v1key
        if not valid:
            self.send_response(400);self.end_headers();self.wfile.write(b'{}');return
        number=p['out_trade_no']
        if self.path in ('/mapi.php','/api/pay/create'):
            self.orders[number] = dict(p,trade_no='G'+uuid.uuid4().hex,trade_status='TRADE_SUCCESS',status='0')
            r=dict(code=0 if v2 else 1,trade_no=self.orders[number]['trade_no'],pay_info='https://checkout.example.test/'+number)
        else:
            order=self.orders[number]
            r={k:order[k] for k in ('out_trade_no','trade_no','money','type','pid','status')}
            r['code']=0 if v2 else 1
        if v2: r=self.signed(r,True)
        if self.tamper: r['sign']='invalid'
        payload=json.dumps(r).encode();self.send_response(200);self.send_header('Content-Type','application/json');self.send_header('Content-Length',str(len(payload)));self.end_headers();self.wfile.write(payload)

class SMTP(socketserver.StreamRequestHandler):
    messages=[]
    authenticated=0
    def handle(self):
        self.connection.settimeout(5)
        self.wfile.write(b'220 test.example.test ESMTP\r\n')
        data=False; body=[]
        try:
            for line in self.rfile:
                if data:
                    if line==b'.\r\n': self.messages.append(b''.join(body));data=False;self.wfile.write(b'250 accepted\r\n')
                    else: body.append(line)
                    continue
                command=line.split(b' ',1)[0].strip().upper()
                if command in (b'EHLO',b'HELO'): self.wfile.write(b'250-test.example.test\r\n250 AUTH PLAIN LOGIN\r\n')
                elif command==b'AUTH': type(self).authenticated+=1;self.wfile.write(b'235 authenticated\r\n')
                elif command in (b'MAIL',b'RCPT',b'RSET',b'NOOP'): self.wfile.write(b'250 OK\r\n')
                elif command==b'DATA': data=True;body=[];self.wfile.write(b'354 End with dot\r\n')
                elif command==b'QUIT': self.wfile.write(b'221 bye\r\n');break
                else: self.wfile.write(b'500 unsupported\r\n')
        except (TimeoutError,ConnectionError,OSError): pass

class Features(base.API):
    @classmethod
    def setUpClass(cls):
        super().setUpClass()
        cls.tmp=tempfile.TemporaryDirectory();Gateway.keydir=pathlib.Path(cls.tmp.name)
        for name in ('merchant','platform'):
            subprocess.run(['openssl','genpkey','-algorithm','RSA','-pkeyopt','rsa_keygen_bits:2048','-out',str(Gateway.keydir/(name+'.pem'))],check=True,capture_output=True)
            subprocess.run(['openssl','pkey','-in',str(Gateway.keydir/(name+'.pem')),'-pubout','-out',str(Gateway.keydir/(name+'.pub'))],check=True,capture_output=True)
        cls.gateway=http.server.ThreadingHTTPServer(('127.0.0.1',0),Gateway)
        cls.smtp=socketserver.ThreadingTCPServer(('127.0.0.1',0),SMTP);cls.smtp.daemon_threads=True
        for server in (cls.gateway,cls.smtp): threading.Thread(target=server.serve_forever,daemon=True).start()
    @classmethod
    def tearDownClass(cls):
        cls.gateway.shutdown();cls.gateway.server_close();cls.smtp.shutdown();cls.smtp.server_close();cls.tmp.cleanup();super().tearDownClass()
    def tearDown(self):
        self.api('PUT','system/setting/mail',{'enabled':False})
        self.api('PUT','system/setting/payment',{'enabled':False,'gatewayUrl':'','merchantId':''})
    def payment_config(self,v2=False):
        cfg=dict(enabled=True,provider='epay',epayVersion='v2' if v2 else 'v1',gatewayUrl='http://127.0.0.1:'+str(self.gateway.server_port),merchantId=Gateway.merchant,merchantKey=Gateway.v1key,enabledPayTypes=['alipay'],notifyUrl='',returnUrl='')
        if v2: cfg.update(merchantPrivateKey=(Gateway.keydir/'merchant.pem').read_text(),platformPublicKey=(Gateway.keydir/'platform.pub').read_text())
        self.api('PUT','system/setting/payment',cfg)
    def order(self):
        payload=dict(outTradeNo='I'+uuid.uuid4().hex,money='12.30',payType='alipay',subject='协议集成测试')
        result=self.api('POST','payment/orders',payload)['data'];return payload,result
    def callback(self,p,expected=200):
        raw=urllib.parse.urlencode(p).encode();status,response=base.request('POST','system-config/payment/notify',raw,raw=True,content_type='application/x-www-form-urlencoded')
        self.assertEqual(status,expected,response);return response
    def test_feature_payment_v1_idempotence_and_amount(self):
        self.payment_config();payload,order=self.order();self.assertEqual(order['amountCents'],1230)
        again=self.api('POST','payment/orders',payload)['data'];self.assertEqual(again['id'],order['id'])
        self.api('POST','payment/orders',dict(payload,money='13.00'),expected=422)
        original=Gateway.orders[order['outTradeNo']]
        p={k:original[k] for k in ('pid','type','out_trade_no','trade_no','money','trade_status')}
        self.callback(Gateway.signed(dict(p,money='99.00')),422)
        self.callback(dict(Gateway.signed(p),sign='0'*32),422)
        self.assertEqual(self.callback(Gateway.signed(p)),b'success');self.assertEqual(self.callback(Gateway.signed(p)),b'success')
        paid=self.api('GET','payment/orders/'+order['outTradeNo'])['data'];self.assertEqual(paid['status'],'paid')
        self.api('POST','payment/orders',payload,token=self.member,expected=403)
    def test_feature_payment_v2_query_and_signature(self):
        self.payment_config(True);payload,order=self.order();Gateway.orders[order['outTradeNo']]['status']='1'
        paid=self.api('POST','payment/orders/'+order['outTradeNo']+'/query')['data'];self.assertEqual(paid['status'],'paid')
        _,order2=self.order();original=Gateway.orders[order2['outTradeNo']];p={k:original[k] for k in ('pid','type','out_trade_no','trade_no','money','trade_status')}
        self.callback(Gateway.signed(p,True))
        expired=Gateway.signed(p,True);expired['timestamp']='1';self.callback(expired,422)
        Gateway.tamper=True
        try: self.api('POST','payment/orders',dict(payload,outTradeNo='I'+uuid.uuid4().hex),expected=422)
        finally: Gateway.tamper=False
    def test_feature_smtp_delivery_and_tls_refusal(self):
        cfg=dict(enabled=True,host='127.0.0.1',port=self.smtp.server_address[1],encryption='none',username='test-smtp',password='smtp-test-password',fromEmail='sender@example.test',fromName='测试发件人',testRecipient='receiver@example.test')
        self.api('PUT','system/setting/mail',cfg);before=len(SMTP.messages)
        r=self.api('POST','system/setting/mail/test',{'subject':'SMTP Integration','body':'Test body'})['data'];self.assertTrue(r['success']);self.assertEqual(len(SMTP.messages),before+1);self.assertTrue(SMTP.authenticated)
        self.assertIn(b'Test body',SMTP.messages[-1])
        self.api('POST','system/setting/mail/test',{'to':'bad-address'},expected=422)
        self.api('POST','system/setting/mail/test',{},token=self.member,expected=403)
        self.api('PUT','system/setting/mail',{'encryption':'starttls'})
        self.api('POST','system/setting/mail/test',{},expected=422);self.assertEqual(len(SMTP.messages),before+1)
    def redis(self,*args):
        def read(f):
            line=f.readline();kind=line[:1];value=line[1:-2]
            if kind==b'$':
                n=int(value)
                if n==-1:return None
                data=f.read(n);f.read(2);return data
            if kind==b':':return int(value)
            if kind==b'+':return value
            if kind==b'*':return [read(f) for _ in range(int(value))]
            raise RuntimeError(line)
        with socket.create_connection(('127.0.0.1',int(os.environ.get('TEST_REDIS_PORT','16391'))),timeout=3) as s:
            parts=[str(a).encode() for a in args];s.sendall(b'*'+str(len(parts)).encode()+b'\r\n'+b''.join(b'$'+str(len(p)).encode()+b'\r\n'+p+b'\r\n' for p in parts));return read(s.makefile('rb'))
    def test_feature_redis_namespace_and_real_metrics(self):
        self.redis('SET','unrelated:test','keep');self.redis('SET','ruoyi:cache:a/b','value');self.redis('EXPIRE','ruoyi:cache:a/b',120)
        info=self.api('GET','monitor/cache')['data'];self.assertTrue(info['info']['redis_version']);self.assertGreater(info['dbSize'],0)
        names=self.api('GET','monitor/cache/getNames')['data'];self.assertEqual(names[0]['cacheName'],'ruoyi:cache:')
        key=urllib.parse.quote('ruoyi:cache:a/b',safe='');value=self.api('GET','monitor/cache/getValue/ruoyi%3Acache%3A/'+key)['data'];self.assertEqual(value['cacheValue'],'value');self.assertGreater(value['ttl'],0)
        self.api('DELETE','monitor/cache/clearCacheKey/unrelated:test',expected=403)
        self.api('GET','monitor/cache',token=self.member,expected=403)
        self.api('DELETE','monitor/cache/clearCacheAll');self.assertEqual(self.redis('GET','unrelated:test'),b'keep');self.assertIsNone(self.redis('GET','ruoyi:cache:a/b'));self.redis('DEL','unrelated:test')
    def test_feature_jobs_scheduler_manual_and_logs(self):
        cfg=dict(jobName='集成测试心跳',jobGroup='DEFAULT',invokeTarget='system.heartbeat',cronExpression='* * * * * ?',status='1',concurrent='1',misfirePolicy='3',remark='')
        self.api('POST','monitor/job',dict(cfg,invokeTarget='unsupported.task'),expected=422)
        self.api('POST','monitor/job',cfg,token=self.member,expected=403)
        jid=self.api('POST','monitor/job',cfg)['data']
        try:
            self.api('PUT','monitor/job/run',{'jobId':jid});logs=self.api('GET','monitor/jobLog/list?jobId='+str(jid));self.assertEqual(logs['total'],1);self.assertEqual(logs['rows'][0]['status'],'0')
            self.api('PUT','monitor/job/changeStatus',{'jobId':jid,'status':'0'})
            for _ in range(30):
                time.sleep(.2)
                if self.api('GET','monitor/jobLog/list?jobId='+str(jid))['total']>=2:break
            self.api('PUT','monitor/job/changeStatus',{'jobId':jid,'status':'1'})
            count=self.api('GET','monitor/jobLog/list?jobId='+str(jid))['total'];self.assertGreaterEqual(count,2);time.sleep(1.2);self.assertEqual(self.api('GET','monitor/jobLog/list?jobId='+str(jid))['total'],count)
            self.api('PUT','monitor/job',dict(cfg,jobId=jid,invokeTarget='cache.ping'));self.api('PUT','monitor/job/run',{'jobId':jid})
            export=self.api('POST','monitor/job/export',{},raw=True);self.assertIn('集成测试心跳',export.decode('utf-8-sig'))
            filtered=self.api('POST','monitor/job/export',{'jobName':'no-such-job'},raw=True);self.assertNotIn('集成测试心跳',filtered.decode('utf-8-sig'))
            self.assertEqual(self.api('GET','monitor/jobLog/list?params%5BendTime%5D=2000-01-01')['total'],0)
        finally:self.api('DELETE','monitor/job/'+str(jid))
    def workbook(self,rows):
        template=self.api('POST','system/user/importTemplate',{},raw=True)
        headers=['登录名称','用户昵称','部门编号','邮箱','手机号码','性别','状态','初始密码']
        xml=['<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>']
        for n,row in enumerate([headers]+rows,1):
            xml.append('<row r="'+str(n)+'">')
            for col,value in enumerate(row):xml.append(f'<c r="{chr(65+col)}{n}" t="inlineStr"><is><t>{escape(str(value))}</t></is></c>')
            xml.append('</row>')
        xml.append('</sheetData></worksheet>');out=io.BytesIO()
        with zipfile.ZipFile(io.BytesIO(template)) as source,zipfile.ZipFile(out,'w',zipfile.ZIP_DEFLATED) as target:
            for name in source.namelist():target.writestr(name,''.join(xml).encode() if name=='xl/worksheets/sheet1.xml' else source.read(name))
        return out.getvalue()
    def upload_workbook(self,rows,update=False,expected=200,token=None):
        data=self.workbook(rows);boundary='----features'+uuid.uuid4().hex
        payload=(f'--{boundary}\r\nContent-Disposition: form-data; name="file"; filename="users.xlsx"\r\nContent-Type: application/vnd.openxmlformats-officedocument.spreadsheetml.sheet\r\n\r\n').encode()+data+(f'\r\n--{boundary}--\r\n').encode()
        return self.api('POST','system/user/importData?updateSupport='+str(int(update)),payload,token=token,expected=expected,content_type='multipart/form-data; boundary='+boundary)
    def test_feature_excel_enforces_data_scope_with_import_permission(self):
        menus=self.api('GET','system/menu/list')['data']
        extra=[m['menuId'] for m in menus if m.get('perms') in ('system:user:import','system:user:add')]
        self.assertTrue(extra)
        self.api('PUT','system/role',{'roleId':self.roleid,'roleName':'导入权限测试','menuIds':self.menuids+extra})
        own=[self.users[0][1],'本人导入修改',103,'','', '0','0','']
        other=[self.users[1][1],'跨范围修改',103,'','', '0','0','']
        try:
            self.upload_workbook([own],True,token=self.member)
            self.upload_workbook([own,other],True,token=self.member,expected=403)
            own[2]=104
            self.upload_workbook([own],True,token=self.member,expected=403)
        finally:
            self.api('PUT','system/role',{'roleId':self.roleid,'roleName':'测试本人范围','menuIds':self.menuids})

    def test_feature_excel_atomic_import_and_updates(self):
        name='xlsx'+uuid.uuid4().hex[:12];password='Initial-test-'+uuid.uuid4().hex
        row=[name,'工作簿用户',103,'xlsx@example.test','13800138000','0','0',password]
        bad=list(row);bad[0]+='bad';bad[2]=9999999
        self.upload_workbook([row,bad],expected=422)
        self.assertEqual(self.api('GET','system/user/list?userName='+name)['total'],0)
        result=self.upload_workbook([row]);self.assertEqual(result['data']['created'],1)
        user=self.api('GET','system/user/list?userName='+name)['rows'][0]
        try:
            self.upload_workbook([row],expected=422)
            row[1]='更新昵称';row[7]=''
            self.assertEqual(self.upload_workbook([row],True)['data']['updated'],1)
            self.assertEqual(base.request('POST','login',{'username':name,'password':password})[0],200)
            self.upload_workbook([row],True,expected=403,token=self.member)
            admin=list(row);admin[0]='admin';self.upload_workbook([admin],True,expected=403)
        finally:self.api('DELETE','system/user/'+str(user['userId']))

if __name__=='__main__':unittest.main(verbosity=2)
