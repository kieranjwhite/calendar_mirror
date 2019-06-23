from collections import namedtuple
from papirus import PapirusTextPos

import json
import socketserver

render_lookups={
    'AddText': lambda p, text, pos, size, ident: p.AddText(text, pos[0], pos[1], size, ident),
    'UpdateText': lambda p, ident, new_text: p.UpdateText(ident, new_text),
    'RemoveText': lambda p, ident: p.RemoveText(ident),
    'Clear': lambda p: p.Clear(),
    'WriteAll': lambda p: p.WriteAll(),
}

class MyTCPHandler(socketserver.StreamRequestHandler):
    def invokeop(self, page, op):
        if isinstance(op, str):
            render_lookups[op](page)
        else:
            enum_name=[*op][0]
            render_lookups[enum_name](*([page]+op[enum_name]))
            
    def handle(self):
        page = PapirusTextPos(False)
        
        line=self.rfile.readline()
        while line!=b'':
            print("line: "+repr(line))
            op=json.loads(line.decode('utf-8'))
            #op=json.loads(line.decode('utf-8'), object_hook=lambda d: namedtuple('X', d.keys())(*d.values()))
            print("op: "+repr(op)+" type: "+str(type(op)))
            #import pdb; pdb.set_trace()
            self.invokeop(page, op)
            line=self.rfile.readline()

        print("finished with connection")

if __name__ == "__main__":
    HOST, PORT = "localhost", 6029
    server = socketserver.TCPServer((HOST, PORT), MyTCPHandler)
    server.serve_forever()
