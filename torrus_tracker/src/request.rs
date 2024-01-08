pub trait TrackerRequestBuilder {
    fn build(self) -> impl TrackerRequest;
}

pub trait TrackerRequest {}
