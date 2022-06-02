use coap_lite::{CoapRequest, RequestType, MessageType};
use std::net::{SocketAddr, UdpSocket};

pub struct Connection {
    socket: UdpSocket,
    request: CoapRequest<SocketAddr>,
    token: u16
}

impl Connection {
    pub fn new() -> Connection {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let request: CoapRequest<SocketAddr> = CoapRequest::new();

        Connection { socket, request, token: 0 }
    }

    pub fn send(&mut self, rtype: RequestType, addr: &str, path: &str, message_id: u16, payload: Vec<u8>) {
        self.request.set_method(rtype);
        self.request.set_path(path);
        
        self.token += 1;
        self.request.message.set_token(self.token.to_le_bytes().to_vec());
        self.request.message.payload = payload;
        self.request.message.header.message_id = message_id;
        self.request.message.header.set_type(MessageType::Confirmable);

        let packet = self.request.message.to_bytes().unwrap();
        self.socket
            .send_to(&packet[..], addr)
            .expect("Could not send the data");
    }
}
