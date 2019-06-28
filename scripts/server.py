#!/usr/bin/python3

from collections import namedtuple
from papirus import PapirusTextPos

import json
import socketserver
import sys
import signal

def signal_handler(sig, frame):
    #client will handle SIGINT for us and send a QuitWhenDone message
    pass

SERVER_PREFIX="Server: "

quit_flag=False
def make_quittable():
    global quit_flag
    if not(quit_flag):
        log("setting quit flag")
        quit_flag=True
        signal.signal(signal.SIGINT, signal_handler)

render_lookups={
    'AddText': lambda p, text, pos, size, ident: p.AddText(text, pos[0], pos[1], size, ident),
    'UpdateText': lambda p, ident, new_text: p.UpdateText(ident, new_text),
    'RemoveText': lambda p, ident: p.RemoveText(ident),
    'Clear': lambda p: p.Clear(),
    'WriteAll': lambda p, partial_update: p.WriteAll(partial_update),
    'QuitWhenDone': lambda p: make_quittable(),
}

def log(msg):
    print(SERVER_PREFIX+msg)

class MyTCPHandler(socketserver.StreamRequestHandler):
    def invokeop(self, page, op):
        if isinstance(op, str):
            render_lookups[op](page)
        else:
            enum_name=[*op][0]
            if isinstance(op[enum_name], list):
                #assumes we're receiving a tuple struct with multiple elements - the elements are serialised by serde as a list
                render_lookups[enum_name](*([page]+op[enum_name]))
            else:
                #assumes we're receiving a tuple struct with one element - the element serialised by serde as a scalar
                #import pdb; pdb.set_trace()
                render_lookups[enum_name](page, op[enum_name])
                
    def handle(self):
        page = PapirusTextPos(False)
        
        line=self.rfile.readline()
        while line!=b'':
            log("line: "+repr(line))
            op=json.loads(line.decode('utf-8'))
            #op=json.loads(line.decode('utf-8'), object_hook=lambda d: namedtuple('X', d.keys())(*d.values()))
            log("op: "+repr(op)+" type: "+str(type(op)))
            #import pdb; pdb.set_trace()
            self.invokeop(page, op)
            line=self.rfile.readline()
            
        log("finished with connection")

if __name__ == "__main__":
    HOST, PORT = "localhost", 6029
    server=socketserver.TCPServer((HOST, PORT), MyTCPHandler)

    while quit_flag==False:
        server.handle_request()
    server.server_close()

    log("quitting")


