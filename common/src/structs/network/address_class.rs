use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

// What kind of network an address belongs to, as far as reaching it from the internet goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressClass {
    Public,
    Private,
    Cgnat,
    Loopback,
    LinkLocal,
    Reserved,
}

impl AddressClass {
    // IPv4-mapped IPv6 addresses take the class of the IPv4 address they carry.
    pub fn of(address: IpAddr) -> Self {
        match address.to_canonical() {
            IpAddr::V4(v4) => Self::of_v4(v4),
            IpAddr::V6(v6) => Self::of_v6(v6),
        }
    }

    pub fn is_public(self) -> bool {
        self == Self::Public
    }

    fn of_v4(a: Ipv4Addr) -> Self {
        let o = a.octets();

        if a.is_loopback() {
            Self::Loopback
        } else if a.is_private() {
            Self::Private
        } else if a.is_link_local() {
            Self::LinkLocal
        } else if o[0] == 100 && (o[1] & 0xc0) == 0x40 {
            Self::Cgnat
        } else if a.is_unspecified()
            || a.is_broadcast()
            || a.is_multicast()
            || a.is_documentation()
            || o[0] == 0
            || (o[0] == 192 && o[1] == 0 && o[2] == 0)
            || (o[0] == 198 && (o[1] & 0xfe) == 18)
            || o[0] >= 240
        {
            Self::Reserved
        } else {
            Self::Public
        }
    }

    fn embedded_v4(high: u16, low: u16) -> Ipv4Addr {
        let [a, b] = high.to_be_bytes();
        let [c, d] = low.to_be_bytes();
        Ipv4Addr::new(a, b, c, d)
    }

    fn of_v6(a: Ipv6Addr) -> Self {
        let s = a.segments();

        if a.is_loopback() {
            Self::Loopback
        } else if (s[0] & 0xfe00) == 0xfc00 {
            Self::Private
        } else if (s[0] & 0xffc0) == 0xfe80 {
            Self::LinkLocal
        } else if a.is_unspecified() || a.is_multicast() || (s[0] == 0x2001 && s[1] == 0x0db8) {
            Self::Reserved
        } else if s[..6].iter().all(|segment| *segment == 0) || (s[0] == 0x0100 && s[1..4] == [0, 0, 0]) {
            // IPv4-compatible (::/96, deprecated) and the discard prefix (100::/64).
            Self::Reserved
        } else if s[0] == 0x0064 && s[1] == 0xff9b {
            // NAT64: a translator forwards to the IPv4 address in the low 32 bits. The
            // local-use prefix (64:ff9b:1::/48) has no public meaning.
            if s[2..6].iter().all(|segment| *segment == 0) {
                Self::of_v4(Self::embedded_v4(s[6], s[7]))
            } else {
                Self::Reserved
            }
        } else if s[0] == 0x2002 {
            // 6to4: the IPv4 address sits in bits 16 to 48.
            Self::of_v4(Self::embedded_v4(s[1], s[2]))
        } else {
            Self::Public
        }
    }
}
