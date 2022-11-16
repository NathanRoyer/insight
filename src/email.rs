use dns_parser::Packet;
use dns_parser::Builder;
use dns_parser::QueryType;
use dns_parser::QueryClass;
use dns_parser::rdata::RData;

use lettre::Message;
use lettre::message::Mailbox;
use lettre::message::dkim::DkimConfig;
use lettre::message::dkim::DkimSigningKey;
use lettre::message::dkim::DkimSigningAlgorithm::Rsa as DkimRsa;
use lettre::transport::smtp::client::SmtpConnection;
use lettre::transport::smtp::client::TlsParameters;
use lettre::transport::smtp::commands::*;
use lettre::transport::smtp::extension::ClientId;
use lettre::transport::smtp::SMTP_PORT;

use rsa::pkcs8::EncodePublicKey;
use rsa::pkcs1::EncodeRsaPrivateKey;
use rsa::pkcs1::LineEnding;
use rsa::RsaPrivateKey;
use rsa::RsaPublicKey;

use base64::encode;

use std::net::UdpSocket;
use std::thread::JoinHandle;
use std::thread;
use std::fs::write;
use std::fs::read_to_string;
use std::sync::mpsc;
use std::sync::mpsc::SyncSender;
use std::time::Duration;

pub struct EmailSender {
    dkim_config: DkimConfig,
    dns_id: u16,
    sender_address: String,
    domain_name: String,
}

fn dns_mx_resolve(name: &str, req_id: u16) -> Option<String> {
    let mut query = Builder::new_query(req_id, false);
    query.add_question(name, false, QueryType::MX, QueryClass::IN);
    let query = query.build().ok()?;

    let mut recv_buf = vec![0; 1024 as usize];

    let socket = UdpSocket::bind(("0.0.0.0", 0)).ok()?;
    socket.set_write_timeout(Some(Duration::new(2, 0))).ok()?;
    socket.set_read_timeout(Some(Duration::new(10, 0))).ok()?;
    socket.connect("1.1.1.1:53").ok()?;
    socket.send(&query).ok()?;

    let (bytes_recvd, _) = socket.recv_from(&mut recv_buf).ok()?;
    recv_buf.resize(bytes_recvd, 0);

    let packet = Packet::parse(&recv_buf).ok()?;

    let mut best = u16::MAX;
    let mut best_index = None;
    for i in 0..packet.answers.len() {
        if let RData::MX(mx) = packet.answers[i].data {
            if mx.preference < best {
                best_index = Some(i);
                best = mx.preference;
            }
        }
    }

    if let RData::MX(mx) = packet.answers[best_index?].data {
        Some(mx.exchange.to_string())
    } else {
        None
    }
}

impl EmailSender {
    pub fn new(
        sender_address: String,
        domain_name: String,
        dkim_selector: String,
        dkim_private_key_path: String,
        dkim_txt_path: String,
    ) -> Self {
        let mut dkim_key = read_to_string(&dkim_private_key_path);

        if let Err(_) = dkim_key {
            let mut rng = rand::thread_rng();
            let private_key = RsaPrivateKey::new(&mut rng, 1024).expect("failed to generate a DKIM key");
            let public_key = RsaPublicKey::from(&private_key);
            let document = public_key.to_public_key_der()
                .expect("failed to get pubkey as DER sequence");

            let base64 = encode(document.as_bytes());

            let dkim_txt = format!("{}._domainkey.{}\nv=DKIM1; k=rsa; p={}", &dkim_selector, &domain_name, base64);
            write(&dkim_txt_path, dkim_txt).expect("failed to save txt records");

            private_key
                .write_pkcs1_pem_file(&dkim_private_key_path, LineEnding::default())
                .expect("failed to write DKIM key");

            dkim_key = read_to_string(&dkim_private_key_path);
        }

        let dkim_key = DkimSigningKey::new(&dkim_key.unwrap(), DkimRsa).unwrap();

        let dkim_config = DkimConfig::default_config(
            dkim_selector,
            domain_name.clone(),
            dkim_key,
        );

        Self {
            dkim_config,
            dns_id: 0,
            sender_address,
            domain_name,
        }
    }

    pub fn try_send_email(
        &mut self,
        to_email: &str,
        code: &str,
    ) -> Option<()> {
        let email_domain = to_email.split("@").last()?;

        let smtp_server = dns_mx_resolve(email_domain, self.dns_id)?;
        let smtp_server = smtp_server.as_str();
        self.dns_id += 1;

        // println!("got dns");

        // println!("got best dns: {}", smtp_server);

        let body = format!(r#"Hey,

Your email address is being used on https://{}/ as an authentication method.

If you're aware of this, here is the code you'll need to enter: {}
"#,
            self.domain_name,
            code,
        );

        let mailbox = Mailbox::try_from((&self.domain_name, &self.sender_address)).unwrap();
        let mut message = Message::builder()
            .from(mailbox)
            .to(to_email.parse().ok()?)
            .subject("Authentication Code")
            .message_id(None)
            .body(body)
            .ok()?;

        message.sign(&self.dkim_config);
        let envelope = message.envelope();
        let message = message.formatted();

        // println!("trying to connect");

        let tls_params = TlsParameters::new(smtp_server.into()).ok()?;

        let hello = ClientId::Domain(self.domain_name.clone());
        let address = (smtp_server, SMTP_PORT);
        let mut client = SmtpConnection::connect(&address, None, &hello, None, None).ok()?;

        // println!("connected");

        if client.can_starttls() {
            client.starttls(&tls_params, &hello).ok()?;
        }

        // println!("secured");

        // println!("{:?}", client.server_info());
        let _response = client.send(&envelope, &message);
        // println!("{:?}", _response);
        // println!("cp3");
        client.command(Quit).ok()?;

        println!("sent code {} to {}", code, to_email);

        Some(())
    }
}

pub type ChannelMsg = (String, String);
pub type Mailer = SyncSender<ChannelMsg>;

pub fn spawn_email_thread() -> (JoinHandle<()>, Mailer) {
    let (sender, receiver) = mpsc::sync_channel::<ChannelMsg>(1024);

    let handle = thread::spawn(move || {
        let mut email_sender = EmailSender::new(
            format!("insight@{}", crate::DOMAIN_NAME),
            crate::DOMAIN_NAME.into(),
            crate::DKIM_SELECTOR.into(),
            crate::DKIM_PRIVATE_KEY_PATH.into(),
            crate::DNS_TXT_PATH.into(),
        );

        while let Ok((to_email, code)) = receiver.recv() {
            if let None = email_sender.try_send_email(&to_email, &code) {
                println!("failed to send code {} to {}", code, to_email);
            }
        }

        panic!("mail thread has stopped");
    });

    (handle, sender)
}

