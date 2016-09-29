trait Provider<T, Request, Response> {
    type Item: Borrow<T> + BorrowMut<T>;

    fn provide(&self, manager: &ProviderManager<Request, Response>, request: &Request) -> Result<Self::Item, ProviderError>;

    /// Persist any side effects onto the response or other providers if necessary. It is
    /// guaranteed that this will be run before finalizing any providers requested in
    /// `self.provide`.
    fn finalize(&self, manager: &mut ProviderManager<Request, Response>, item: Self::Item, response: &mut Response) -> Result<(), ProviderError> {
        Ok(())
    }
}

trait ProviderDecorator<T, Request, Response> {
    fn provide_decoration(&self, manager: &ProviderManager<Request, Response>, item: &mut T) -> Result<(), ProviderError>;
    fn finalize_decoration(&self, manager: &mut ProviderManager<Request, Response>, item: &mut T) -> Result<(), ProviderError>;
}

struct ProviderManager<Request, Response> {
    _marker: PhantomData<(Request, Response)>,
}

impl<Request, Response> ProviderManager {
    /// Returns a reference to the registered provider for T
    pub fn get<T>(&self) -> Result<&T, ProviderError> {
        unimplemented!();
    }

    /// Returns a unique reference to the registered provider for T
    pub fn get_mut<T>(&mut self) -> Result<&mut T, ProviderError> {
        unimplemented!();
    }

    /// Registers a new provider for T. Returns an error if a provider
    /// has already been registered for that type.
    pub fn register<T, U>(&mut self, provider: U) -> Result<Self, ProviderAlreadyRegistered>
    where U: Provider<T, Request, Response> {
        unimplemented!();
    }

    pub fn override_provider<T, U>(&mut self, provider: U) -> Result<Self, ProviderNotRegistered>
    where U: Provider<T, Request, Response> {
        unimplemented!();
    }

    /// Applies a decoration to a provider, calling the original provider and then passing it to
    /// the decoration
    pub fn decorate_provider<T, U>(&mut self, provider: U) -> Self
    where U: ProviderDecorator<T, Request, Response> {
        unimplemented!();
    }
}
