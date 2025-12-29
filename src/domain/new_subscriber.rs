use crate::domain::{subscriber_name::SuscriberName, subscriber_email::SubscriberEmail};

pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SuscriberName,
}
