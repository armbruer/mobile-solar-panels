use coap_lite::{error::MessageError, CoapRequest, CoapResponse, MessageType, Packet, RequestType};
use std::{
    io::ErrorKind,
    net::{SocketAddr, UdpSocket},
    time::Duration,
};

pub struct Connection {
    socket: UdpSocket,
    token: u16,
    message_id: u16,
}

impl Connection {
    pub fn new() -> Connection {
        let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
        let mut con = Connection {
            socket,
            token: 0,
            message_id: 0,
        };

        con.set_timeout(Some(Duration::from_millis(500)));
        con
    }

    pub fn send(
        &mut self,
        rtype: RequestType,
        addr: &str,
        path: &str,
        payload: Vec<u8>,
    ) -> Option<CoapResponse> {
        log::warn!("We are here!");
        let mut request: CoapRequest<SocketAddr> = CoapRequest::new();

        request.set_method(rtype);
        request.set_path(path);

        self.token += 1;
        self.message_id += 1;
        request.message.set_token(self.token.to_le_bytes().to_vec());
        request.message.payload = payload;
        request.message.header.message_id = self.message_id;
        request.message.header.set_type(MessageType::Confirmable);

        let packet = request.message.to_bytes().unwrap();
        self.socket
            .send_to(&packet[..], addr)
            .expect("Could not send the data");

        log::warn!("We are even further!");
        let returns = self.wait_for_response(request);
        log::warn!("got: {:#?}", returns);
        returns
    }

    fn wait_for_response(&mut self, req: CoapRequest<SocketAddr>) -> Option<CoapResponse> {
        let mut recvd_ack = false;
        let mut recvd_response = false;

        let mut response = None;

        while let Ok(res) = self.recv() {
            log::warn!("wait_for_response: {:#?}", res);
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
                recvd_response = true;
                response = Some(res);
                log::info!("Received reponse: {:#?}", response.as_ref().unwrap());
            }

            if recvd_ack && recvd_response {
                break;
            }
        }

        response
    }

    fn recv(&mut self) -> Result<CoapResponse, MessageError> {
        let mut buf = [0; 1500];

        let (nread, src) = loop {
            let res = self.socket.recv_from(&mut buf);
            log::debug!("recv(): {:#?}", res);
            match res {
                Ok(res) => break res,
                Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                    return Err(MessageError::InvalidTokenLength)
                }
                Err(e) => todo!("{:#?}", e),
            }
        };

        let packet = Packet::from_bytes(&buf[..nread])?;

        let receive_packet = CoapRequest::from_packet(packet, &src);

        Ok(CoapResponse {
            message: receive_packet.message,
        })
    }

    pub fn set_timeout(&mut self, dur: Option<Duration>) {
        self.socket.set_read_timeout(dur).unwrap();
    }
}
