//! Native authority for page-scoped capabilities. Context comes from the engine,
//! never from guest memory, URL parameters, JavaScript or an untrusted IPC claim.
use std::collections::HashMap;
use std::time::{Duration, Instant};
use url::Url;

const MAX_PAGES: usize = 64;
const MAX_GRANTS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PageToken(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Handle(u64);

impl Handle {
    pub fn value(self) -> u64 {
        self.0
    }
    pub fn from_guest(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Capability {
    Diagnostic = 1,
    WorldTracking = 2,
    RecordMovement = 4,
    RecordVideo = 8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InsecureOrigin,
    InvalidUrl,
    StalePage,
    NotTopLevel,
    NotForeground,
    ActivationRequired,
    Unsupported,
    ExpiredHandle,
    WrongCapability,
    LimitExceeded,
}

struct Page {
    origin: String,
    top_level: bool,
    foreground: bool,
    private: bool,
    activation_epoch: u64,
}

/// Minted only by native input routing. Consuming this value cannot grant a
/// second capability without another activation. Approval is a native UI action.
#[derive(Debug)]
pub struct Activation {
    page: PageToken,
    created: Instant,
    epoch: u64,
}

pub struct Broker {
    pages: HashMap<PageToken, Page>,
    tabs: HashMap<(u64, u64), PageToken>,
    grants: HashMap<Handle, (PageToken, Capability)>,
    next: u64,
    supported: u32,
}

impl Broker {
    pub fn new(supported: u32) -> Self {
        Self {
            pages: HashMap::new(),
            tabs: HashMap::new(),
            grants: HashMap::new(),
            next: 1,
            supported,
        }
    }

    fn serial(&mut self) -> Result<u64, Error> {
        let value = self.next;
        self.next = self.next.checked_add(1).ok_or(Error::LimitExceeded)?;
        Ok(value)
    }

    /// Call on every document replacement, including same-origin reloads and
    /// redirects. Invalid destinations revoke the old page before failing closed.
    pub fn navigate(
        &mut self,
        profile: u64,
        tab: u64,
        engine_url: &str,
        top_level: bool,
        private: bool,
    ) -> Result<PageToken, Error> {
        self.close(profile, tab);
        let url = Url::parse(engine_url).map_err(|_| Error::InvalidUrl)?;
        if url.scheme() != "https"
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err(Error::InsecureOrigin);
        }
        if self.pages.len() >= MAX_PAGES {
            return Err(Error::LimitExceeded);
        }
        let token = PageToken(self.serial()?);
        self.pages.insert(
            token,
            Page {
                origin: url.origin().ascii_serialization(),
                top_level,
                foreground: true,
                private,
                activation_epoch: 0,
            },
        );
        self.tabs.insert((profile, tab), token);
        Ok(token)
    }

    pub fn close(&mut self, profile: u64, tab: u64) {
        if let Some(page) = self.tabs.remove(&(profile, tab)) {
            self.pages.remove(&page);
            self.grants.retain(|_, (owner, _)| *owner != page);
        }
    }

    pub fn origin(&self, page: PageToken) -> Result<&str, Error> {
        Ok(&self.pages.get(&page).ok_or(Error::StalePage)?.origin)
    }

    pub fn is_private(&self, page: PageToken) -> Result<bool, Error> {
        Ok(self.pages.get(&page).ok_or(Error::StalePage)?.private)
    }

    pub fn set_foreground(&mut self, page: PageToken, foreground: bool) -> Result<(), Error> {
        let current = self.pages.get_mut(&page).ok_or(Error::StalePage)?;
        current.foreground = foreground;
        if !foreground {
            current.activation_epoch = current
                .activation_epoch
                .checked_add(1)
                .ok_or(Error::LimitExceeded)?;
            self.grants.retain(|_, (owner, _)| *owner != page);
        }
        Ok(())
    }

    fn eligible(&self, page: PageToken) -> Result<(), Error> {
        let page = self.pages.get(&page).ok_or(Error::StalePage)?;
        if !page.top_level {
            return Err(Error::NotTopLevel);
        }
        if !page.foreground {
            return Err(Error::NotForeground);
        }
        Ok(())
    }

    /// Native code calls this after a trusted input event; guests cannot mint it.
    pub fn activate(&self, page: PageToken) -> Result<Activation, Error> {
        self.eligible(page)?;
        Ok(Activation {
            page,
            created: Instant::now(),
            epoch: self
                .pages
                .get(&page)
                .ok_or(Error::StalePage)?
                .activation_epoch,
        })
    }

    /// Call only after affirmative native permission UI. A grant is ephemeral;
    /// private and regular pages both require their own approval.
    pub fn approve(
        &mut self,
        page: PageToken,
        capability: Capability,
        activation: Activation,
    ) -> Result<Handle, Error> {
        self.eligible(page)?;
        if activation.page != page
            || activation.created.elapsed() > Duration::from_secs(30)
            || activation.epoch
                != self
                    .pages
                    .get(&page)
                    .ok_or(Error::StalePage)?
                    .activation_epoch
        {
            return Err(Error::ActivationRequired);
        }
        if self.supported & capability as u32 == 0 {
            return Err(Error::Unsupported);
        }
        if self.grants.len() >= MAX_GRANTS {
            return Err(Error::LimitExceeded);
        }
        let handle = Handle(self.serial()?);
        self.grants.insert(handle, (page, capability));
        Ok(handle)
    }

    /// Caller page identity is captured from the executing engine realm. Never
    /// accept a page token transmitted by the guest as that identity.
    pub fn validate(
        &self,
        caller: PageToken,
        handle: Handle,
        capability: Capability,
    ) -> Result<(), Error> {
        self.eligible(caller)?;
        let &(owner, granted) = self.grants.get(&handle).ok_or(Error::ExpiredHandle)?;
        if owner != caller {
            return Err(Error::ExpiredHandle);
        }
        if granted != capability {
            return Err(Error::WrongCapability);
        }
        Ok(())
    }

    pub fn revoke(&mut self, handle: Handle) {
        self.grants.remove(&handle);
    }
}

/// Bounds-check before native code reads/writes a caller-owned WASM buffer.
pub fn guest_range(
    pointer: u32,
    length: u32,
    memory_size: usize,
    maximum: usize,
) -> Result<std::ops::Range<usize>, Error> {
    let start = usize::try_from(pointer).map_err(|_| Error::LimitExceeded)?;
    let count = usize::try_from(length).map_err(|_| Error::LimitExceeded)?;
    let end = start.checked_add(count).ok_or(Error::LimitExceeded)?;
    if count > maximum || end > memory_size {
        return Err(Error::LimitExceeded);
    }
    Ok(start..end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_profiles_private_tabs_and_revocation_are_isolated() {
        let mut broker = Broker::new(1);
        let a = broker
            .navigate(1, 1, "https://NOT-REAL.games:443/wala/", true, false)
            .unwrap();
        assert_eq!(broker.origin(a), Ok("https://not-real.games"));
        let handle = broker
            .approve(a, Capability::Diagnostic, broker.activate(a).unwrap())
            .unwrap();
        let b = broker
            .navigate(2, 1, "https://not-real.games/", true, true)
            .unwrap();
        assert_eq!(broker.is_private(b), Ok(true));
        assert_eq!(
            broker.validate(b, handle, Capability::Diagnostic),
            Err(Error::ExpiredHandle)
        );
        assert_eq!(broker.validate(a, handle, Capability::Diagnostic), Ok(()));
        assert_eq!(
            broker.validate(a, handle, Capability::RecordVideo),
            Err(Error::WrongCapability)
        );
        broker
            .navigate(1, 1, "https://elsewhere.example/", true, false)
            .unwrap();
        assert_eq!(
            broker.validate(a, handle, Capability::Diagnostic),
            Err(Error::StalePage)
        );
        let c = broker
            .navigate(1, 1, "https://not-real.games/", true, false)
            .unwrap();
        assert_eq!(
            broker.validate(c, handle, Capability::Diagnostic),
            Err(Error::ExpiredHandle)
        );
    }

    #[test]
    fn failure_and_backgrounding_revoke_existing_authority() {
        let mut broker = Broker::new(1);
        let page = broker
            .navigate(1, 1, "https://example.org/", true, false)
            .unwrap();
        let handle = broker
            .approve(page, Capability::Diagnostic, broker.activate(page).unwrap())
            .unwrap();
        let activation = broker.activate(page).unwrap();
        broker.set_foreground(page, false).unwrap();
        assert_eq!(
            broker.validate(page, handle, Capability::Diagnostic),
            Err(Error::NotForeground)
        );
        broker.set_foreground(page, true).unwrap();
        assert_eq!(
            broker.approve(page, Capability::Diagnostic, activation),
            Err(Error::ActivationRequired)
        );
        assert_eq!(
            broker.validate(page, handle, Capability::Diagnostic),
            Err(Error::ExpiredHandle)
        );
        assert_eq!(
            broker.navigate(1, 1, "http://example.org/", true, false),
            Err(Error::InsecureOrigin)
        );
        assert_eq!(broker.origin(page), Err(Error::StalePage));
    }

    #[test]
    fn permission_checks_reject_iframes_wrong_activation_and_unavailable_features() {
        let mut broker = Broker::new(1);
        let page = broker
            .navigate(1, 1, "https://example.org/", true, false)
            .unwrap();
        let frame = broker
            .navigate(1, 2, "https://frame.example.org/", false, false)
            .unwrap();
        assert_eq!(broker.activate(frame).unwrap_err(), Error::NotTopLevel);
        let other = broker
            .navigate(1, 3, "https://example.org/", true, false)
            .unwrap();
        assert_eq!(
            broker.approve(
                other,
                Capability::Diagnostic,
                broker.activate(page).unwrap()
            ),
            Err(Error::ActivationRequired)
        );
        assert_eq!(
            broker.approve(
                page,
                Capability::WorldTracking,
                broker.activate(page).unwrap()
            ),
            Err(Error::Unsupported)
        );
        let old = Activation {
            page,
            created: Instant::now() - Duration::from_secs(31),
            epoch: 0,
        };
        assert_eq!(
            broker.approve(page, Capability::Diagnostic, old),
            Err(Error::ActivationRequired)
        );
        assert_eq!(
            broker.navigate(1, 4, "https://user:pass@example.org/", true, false),
            Err(Error::InsecureOrigin)
        );
    }

    #[test]
    fn authority_and_buffers_are_bounded() {
        let mut broker = Broker::new(1);
        for tab in 0..MAX_PAGES as u64 {
            broker
                .navigate(1, tab, "https://example.org/", true, false)
                .unwrap();
        }
        assert_eq!(
            broker.navigate(1, 999, "https://example.org/", true, false),
            Err(Error::LimitExceeded)
        );
        assert_eq!(guest_range(16, 16, 32, 16), Ok(16..32));
        assert_eq!(guest_range(32, 0, 32, 16), Ok(32..32));
        assert_eq!(
            guest_range(u32::MAX, u32::MAX, 32, 16),
            Err(Error::LimitExceeded)
        );
        assert_eq!(guest_range(0, 17, 32, 16), Err(Error::LimitExceeded));
        let page = *broker.pages.keys().next().unwrap();
        for _ in 0..MAX_GRANTS {
            broker
                .approve(page, Capability::Diagnostic, broker.activate(page).unwrap())
                .unwrap();
        }
        assert_eq!(
            broker.approve(page, Capability::Diagnostic, broker.activate(page).unwrap()),
            Err(Error::LimitExceeded)
        );
    }
}
