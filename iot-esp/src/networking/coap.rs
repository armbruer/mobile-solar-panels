use coap_lite::{CoapRequest, CoapResponse, MessageType, Packet, RequestType};
use std::{
    io::ErrorKind,
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    time::Duration,
};

pub struct Connection {
    socket: UdpSocket,
    token: u16,
    message_id: u16,
}

#[derive(Debug)]
pub enum CoapError {
    ConnectionError(std::io::Error), // Socket error occured
    TimedOut,                        // Did not receive a response in time
    InvalidResponse,
}

impl Connection {
    pub fn new() -> Connection {
        let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
        let mut con = Connection {
            socket,
            token: 0,
            message_id: 0,
        };

        con.set_timeout(Some(Duration::from_secs(2)));
        con
    }

    pub fn request<A: ToSocketAddrs>(
        &mut self,
        rtype: RequestType,
        addr: A,
        path: &str,
        payload: Vec<u8>,
    ) -> Result<CoapResponse, CoapError> {
        let addr = addr.to_socket_addrs().unwrap().collect::<Vec<SocketAddr>>()[0];
        let mut request: CoapRequest<SocketAddr> = CoapRequest::new();

        request.set_method(rtype);
        request.set_path(path);

        self.token = self.token.wrapping_add(1);
        self.message_id = self.message_id.wrapping_add(1);

        request.message.set_token(self.token.to_le_bytes().to_vec());
        request.message.header.message_id = self.message_id;
        request.message.header.set_type(MessageType::Confirmable);

        request.message.payload = payload;

        let packet = request.message.to_bytes().unwrap();
        self.socket
            .send_to(&packet[..], addr)
            .map_err(CoapError::ConnectionError)?;
        log::info!("Sent request packet");

        self.wait_for_response(request, addr)
    }

    fn send_ack(&mut self, resp: &CoapResponse, addr: SocketAddr) {
        let mut request: CoapRequest<SocketAddr> = CoapRequest::new();
        request
            .message
            .header
            .set_type(MessageType::Acknowledgement);
        request.message.header.message_id = resp.message.header.message_id;

        let packet = request.message.to_bytes().unwrap();
        self.socket
            .send_to(&packet[..], addr)
            .expect("Could not send the data");
    }

    fn wait_for_response(
        &mut self,
        req: CoapRequest<SocketAddr>,
        addr: SocketAddr,
    ) -> Result<CoapResponse, CoapError> {
        let mut recvd_ack = false;
        let mut recvd_response = false;

        loop {
            log::info!("Waiting for packet");
            let res = self.recv(addr)?;
            log::info!("Got packet");

            if res.message.header.get_type() == MessageType::Acknowledgement
                && res.message.header.message_id == req.message.header.message_id
            {
                recvd_ack = true;
                log::info!("Received ack: {:#?}", res.message.header.message_id);
            }

            // TODO handle case: acknowledge missing, but response received
            if res.message.get_token() == req.message.get_token()
                && res.message.header.get_type() == MessageType::Confirmable
            {
                self.send_ack(&res, addr);
                recvd_response = true;
                log::info!("Received reponse: {:#?}", &res);
                return Ok(res);
            }
        }
    }

    fn recv(&mut self, _addr: SocketAddr) -> Result<CoapResponse, CoapError> {
        let mut buf = [0; 1500];

        // Src not checked since for private WiFi network and without authentication this doesn't matter
        let (nread, _src) = match self.socket.recv_from(&mut buf) {
            Ok(res) => res,
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                return Err(CoapError::TimedOut)
            }
            Err(e) => return Err(CoapError::ConnectionError(e)),
        };

        let packet = Packet::from_bytes(&buf[..nread]).map_err(|_| CoapError::InvalidResponse)?;
        Ok(CoapResponse { message: packet })
    }

    pub fn set_timeout(&mut self, dur: Option<Duration>) {
        self.socket.set_read_timeout(dur).unwrap();
    }
}
