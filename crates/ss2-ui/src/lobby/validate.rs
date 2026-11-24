//! Reading what the player types on the join screen: a direct address, and an invite code.
//!
//! These are **format checks only**, written from the rules in `docs/DISCOVERY.md` (the network crate is not a dependency of `ss2-ui`). The
//! address rules are the same as `ss2_net::discovery::parse_direct`; the invite check stops at the shape of a code (the alphabet, the length,
//! the padding bits), because the checksum belongs to `ss2_net::discovery::decode_invite`, which the app calls and answers with
//! [`Failure::InvalidInvite`](super::Failure::InvalidInvite) when the code is not one it made.

use std::net::{Ipv4Addr, Ipv6Addr};

/// The port used when a player types an address without one (the same number as `ss2_net::discovery::DEFAULT_PORT`).
pub const DEFAULT_PORT: u16 = 27015;
/// The longest address text taken, in bytes.
pub const MAX_ADDRESS_CHARS: usize = 300;
/// The number of code characters of an invite for an IPv4 address.
pub const INVITE_LEN_V4: usize = 15;
/// The number of code characters of an invite for an IPv6 address.
pub const INVITE_LEN_V6: usize = 34;
/// The most characters (dashes and spaces included) the invite field takes.
pub const MAX_INVITE_CHARS: usize = 60;

const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// The host part of a direct address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostSpec {
    /// An IPv4 address.
    Ipv4(Ipv4Addr),
    /// An IPv6 address.
    Ipv6(Ipv6Addr),
    /// A host name, in lower case, for the app to resolve.
    Name(String),
}

/// An address the player typed, read: the host and the port (the default one when none was typed).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectAddress {
    /// The host.
    pub host: HostSpec,
    /// The port, 1 to 65535.
    pub port: u16,
}

/// Why a typed address was not accepted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddressError {
    /// Nothing typed.
    Empty,
    /// More than [`MAX_ADDRESS_CHARS`] bytes.
    TooLong,
    /// A space or a control character inside the text.
    Whitespace,
    /// A `[` without its `]`, a colon in a name, or what is inside the brackets is not an IPv6 address.
    BadIpv6,
    /// The text after the colon is not a port from 1 to 65535.
    BadPort,
    /// Digits and dots, but not four numbers from 0 to 255.
    BadIpv4,
    /// A name with an empty part, a part over 63 characters, a dash at the start or end of a part, or a character other than letters, digits, dashes.
    BadHostName,
}

impl AddressError {
    /// A sentence for the player.
    pub fn message(self) -> &'static str {
        match self {
            Self::Empty => {
                "Type the host's address, for example 192.168.1.20 or 192.168.1.20:27015"
            }
            Self::TooLong => "That address is too long",
            Self::Whitespace => "An address has no spaces in it",
            Self::BadIpv6 => "An IPv6 address goes in brackets: [::1] or [::1]:27015",
            Self::BadPort => "The port must be a number from 1 to 65535",
            Self::BadIpv4 => "An IPv4 address is four numbers from 0 to 255, like 192.168.1.20",
            Self::BadHostName => "That is not a valid host name (letters, digits, dots, dashes)",
        }
    }
}

fn parse_port(text: &str) -> Option<u16> {
    if text.is_empty() || text.len() > 5 || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse::<u16>().ok().filter(|p| *p != 0)
}

fn valid_host(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 253
        && name.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
}

/// Read what a player typed: `1.2.3.4`, `1.2.3.4:5`, `[::1]`, `[::1]:5`, `::1`, `host`, `host:5`. A missing port is [`DEFAULT_PORT`]. Spaces around the
/// text are ignored; spaces inside it are an error. Never panics, whatever the text.
pub fn parse_address(input: &str) -> Result<DirectAddress, AddressError> {
    let text = input.trim();
    if text.is_empty() {
        return Err(AddressError::Empty);
    }
    if text.len() > MAX_ADDRESS_CHARS {
        return Err(AddressError::TooLong);
    }
    if text.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(AddressError::Whitespace);
    }
    if let Some(rest) = text.strip_prefix('[') {
        let (ip, tail) = rest.split_once(']').ok_or(AddressError::BadIpv6)?;
        let ip: Ipv6Addr = ip.parse().map_err(|_| AddressError::BadIpv6)?;
        let port = match tail {
            "" => DEFAULT_PORT,
            t => parse_port(t.strip_prefix(':').ok_or(AddressError::BadPort)?)
                .ok_or(AddressError::BadPort)?,
        };
        return Ok(DirectAddress {
            host: HostSpec::Ipv6(ip),
            port,
        });
    }
    // A bare IPv6 address (`::1`) has colons of its own: it takes the default port.
    if let Ok(ip) = text.parse::<Ipv6Addr>() {
        return Ok(DirectAddress {
            host: HostSpec::Ipv6(ip),
            port: DEFAULT_PORT,
        });
    }
    let (host, port) = match text.rsplit_once(':') {
        Some((h, p)) => (h, parse_port(p).ok_or(AddressError::BadPort)?),
        None => (text, DEFAULT_PORT),
    };
    if host.contains(':') {
        return Err(AddressError::BadIpv6);
    }
    if let Ok(ip) = host.parse::<Ipv4Addr>() {
        return Ok(DirectAddress {
            host: HostSpec::Ipv4(ip),
            port,
        });
    }
    // Digits and dots that are not an IPv4 address (999.1.1.1, 1.2.3) are not a host name either.
    if host.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return Err(AddressError::BadIpv4);
    }
    let name = host.to_ascii_lowercase();
    if valid_host(&name) {
        Ok(DirectAddress {
            host: HostSpec::Name(name),
            port,
        })
    } else {
        Err(AddressError::BadHostName)
    }
}

/// Why a typed invite code was not accepted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InviteError {
    /// Nothing typed.
    Empty,
    /// A character that is not in the code alphabet (the letters I, L, O are read as 1, 1, 0; U is not used).
    BadCharacter(char),
    /// Fewer characters than a code has so far (a code has 15, or 34 for an IPv6 host; so 16 to 33 are still on the way to 34).
    Incomplete {
        /// The characters so far (dashes and spaces not counted).
        have: usize,
    },
    /// More than 34 characters.
    BadLength {
        /// The characters (dashes and spaces not counted).
        have: usize,
    },
    /// The last character cannot end a real code.
    BadEnding,
}

impl InviteError {
    /// A sentence for the player.
    pub fn message(self) -> String {
        match self {
            Self::Empty => "Type the invite code the host gave you".to_owned(),
            Self::BadCharacter(c) => format!("'{c}' is not used in invite codes"),
            Self::Incomplete { have } => {
                format!("{have} characters so far: a code has {INVITE_LEN_V4} (or {INVITE_LEN_V6})")
            }
            Self::BadLength { .. } => format!(
                "A code has {INVITE_LEN_V4} characters (or {INVITE_LEN_V6}), this has more than that"
            ),
            Self::BadEnding => {
                "That is not a real invite code: check the last characters".to_owned()
            }
        }
    }
}

/// The value of a code character, or `None` outside the alphabet. `O`, `I`, `L` are read as `0`, `1`, `1`; the case does not matter.
fn code_value(c: char) -> Option<u8> {
    let c = match c.to_ascii_uppercase() {
        'O' => '0',
        'I' | 'L' => '1',
        other => other,
    };
    ALPHABET
        .iter()
        .position(|a| *a as char == c)
        .map(|i| i as u8)
}

/// Check the shape of an invite code the player typed and give it in its written form (capitals, groups of four, dashes). Dashes and spaces
/// anywhere are ignored. The checksum is **not** checked here (see the module documentation). Never panics, whatever the text.
pub fn normalize_invite(input: &str) -> Result<String, InviteError> {
    let mut values: Vec<u8> = Vec::new();
    for c in input.chars() {
        if c == '-' || c.is_whitespace() {
            continue;
        }
        let v = code_value(c).ok_or(InviteError::BadCharacter(c))?;
        values.push(v);
        if values.len() > INVITE_LEN_V6 {
            return Err(InviteError::BadLength { have: values.len() });
        }
    }
    let have = values.len();
    if have == 0 {
        return Err(InviteError::Empty);
    }
    // The padding bits of the last character are zero in a real code: 75 bits hold 72 (IPv4), 170 hold 168 (IPv6).
    let pad_mask = match have {
        INVITE_LEN_V4 => 0b0_0111,
        INVITE_LEN_V6 => 0b0_0011,
        n if n < INVITE_LEN_V6 => return Err(InviteError::Incomplete { have }),
        _ => return Err(InviteError::BadLength { have }),
    };
    if values[have - 1] & pad_mask != 0 {
        return Err(InviteError::BadEnding);
    }
    let mut out = String::with_capacity(have + have / 4);
    for (i, v) in values.iter().enumerate() {
        if i > 0 && i % 4 == 0 {
            out.push('-');
        }
        out.push(ALPHABET[usize::from(*v)] as char);
    }
    Ok(out)
}
