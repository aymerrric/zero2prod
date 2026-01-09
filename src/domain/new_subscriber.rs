use crate::domain::{subscriber_email::SubscriberEmail, subscriber_name::SuscriberName};

pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SuscriberName,
}
