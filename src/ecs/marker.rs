pub trait Component: 'static {}

pub trait Resource: 'static {}

pub trait ResourceCore: Resource + 'static {}
