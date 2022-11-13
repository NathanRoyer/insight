use rsdns::clients::std::Client as DnsClient;
use rsdns::clients::ClientConfig;
use rsdns::records::data::Mx;
use rsdns::constants::Class::In as Internet;

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

use std::net::SocketAddr;
use std::thread::JoinHandle;
use std::thread;
use std::fs::write;
use std::fs::read_to_string;
use std::sync::mpsc;
use std::sync::mpsc::SyncSender;
use std::str::FromStr;

pub struct EmailSender {
    dkim_config: DkimConfig,
    dns_client: DnsClient,
    sender_address: String,
    domain_name: String,
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

        let nameserver = SocketAddr::from_str("8.8.8.8:53").unwrap();
        let dns_config = ClientConfig::with_nameserver(nameserver);
        let dns_client = DnsClient::new(dns_config).unwrap();

        Self {
            dkim_config,
            dns_client,
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

        let dns_records = self.dns_client.query_rrset::<Mx>(
            email_domain,
            Internet,
        ).ok()?.rdata;

        // println!("got dns");

        let smtp_server = {
            let best = dns_records.iter().min_by(|a, b| a.preference.cmp(&b.preference))?;
            let mut best = best.exchange.as_str();
            if best.ends_with(".") {
                best = &best[..best.len() - 1];
            }
            best
        };

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

