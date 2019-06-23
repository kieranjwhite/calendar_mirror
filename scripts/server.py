from collections import namedtuple

import json
import socketserver

class MyTCPHandler(socketserver.StreamRequestHandler):

    def handle(self):
        line=self.rfile.readline()
        while line!=b'':
            print("line: "+repr(line))
            op=json.loads(line, object_hook=lambda d: namedtuple('X', d.keys())(*d.values()))
            print("op: "+repr(op))
            line=self.rfile.readline()

        print("finished with connection")

if __name__ == "__main__":
    HOST, PORT = "localhost", 6028
    server = socketserver.TCPServer((HOST, PORT), MyTCPHandler)
    server.serve_forever()
