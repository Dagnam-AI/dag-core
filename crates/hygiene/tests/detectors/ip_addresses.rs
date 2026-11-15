//! `find_ip_addresses` and `is_ip_address`.
use dagnam_hygiene::{find_ip_addresses, is_ip_address};

use super::{assert_finds, assert_misses};

#[test]
fn the_reference_addresses_are_found_exactly() {
    for (index, (template, value)) in [
        ("from {} at noon", "192.168.1.20"),
        ("resolver {}.", "8.8.8.8"),
        ("curl {}:8080/x", "10.0.0.7"),
        // A URL host, a path and after `@` are addresses, not versions.
        ("call http://{}:8080/api", "203.0.113.7"),
        ("open https://{}/login", "10.1.2.3"),
        ("path /srv/{} now", "192.168.1.20"),
        ("ssh admin@{}", "192.168.1.21"),
        ("addr {} up", "2001:db8::1"),
        ("link {} up", "fe80::1ff:fe23:4567:890a"),
        ("mapped {} up", "::ffff:192.0.2.128"),
    ]
    .into_iter()
    .enumerate()
    {
        assert_finds(
            find_ip_addresses,
            &format!("positive {index}"),
            template,
            value,
        );
    }
}

#[test]
fn the_reference_near_misses_are_not_found() {
    for (index, text) in [
        "version 1.2.3 and v1.2.3.4 and 1.2.3.4.5",
        "999.1.1.1 and 256.0.0.1",
        "loopback 127.0.0.1, any 0.0.0.0, ::1 and ::",
        "at 12:30:45, MAC 00:1A:2B:3C:4D:5E",
        "std::vector, a :: b, cafe::bad, Foo::Bar",
        "numpy: a[::2], b[1::2], c[3::], d[10::20], e[::-1], arr[i::j]",
        "x = y[12::34]; z = w[::100]",
        "Mozilla/5.0 Chrome/120.0.0.0 Safari/537.36, .NET Framework 4.8.1.0",
        "firmware 2.1.0.4, version 10.2.3.4, build 3.2.1.9, v 1.2.3.4",
        "agent MyApp/2.3.4.5 (Linux; x64) curl/8.4.0.1",
        "Chrome/120.1.2.3 at the start of the text",
    ]
    .into_iter()
    .enumerate()
    {
        assert_misses(find_ip_addresses, &format!("near miss {index}"), text);
    }
}

#[test]
fn an_address_inside_a_mapped_address_is_one_finding() {
    // The IPv4 candidate and the IPv6 candidate overlap; the union is the IPv6 span.
    let text = "x ::ffff:192.0.2.128 y";
    assert_eq!(find_ip_addresses(text), vec![(2, 20)]);
}

#[test]
fn addresses_of_both_families_are_found_in_order() {
    let text = "10.1.2.3, 2001:db8::1 and 8.8.8.8";
    assert_eq!(find_ip_addresses(text), vec![(0, 8), (10, 21), (26, 33)]);
}

#[test]
fn a_candidate_is_refused_by_the_character_before_and_after() {
    assert_misses(find_ip_addresses, "word before", "x10.1.2.3");
    assert_misses(find_ip_addresses, "dot before", ".10.1.2.3");
    assert_misses(find_ip_addresses, "word after", "10.1.2.3x");
    assert_misses(find_ip_addresses, "dot and word after", "10.1.2.3.x");
    assert_finds(
        find_ip_addresses,
        "dot and space after",
        "{}. ok",
        "10.1.2.3",
    );
    assert_finds(find_ip_addresses, "dot at the end", "{}.", "10.1.2.3");
    assert_misses(find_ip_addresses, "colon before ipv6", ":2001:db8::1x");
    assert_misses(find_ip_addresses, "colon after ipv6", "2001:db8::1:");
    assert_misses(
        find_ip_addresses,
        "dot and word after ipv6",
        "2001:db8::1.a",
    );
    assert_finds(find_ip_addresses, "dot after ipv6", "{}.", "2001:db8::1");
}

#[test]
fn a_candidate_in_another_script_is_no_address() {
    // `\d` takes the digits, the parser refuses them.
    assert_misses(find_ip_addresses, "fullwidth", "\u{ff11}.2.3.4");
}

#[test]
fn a_refused_start_is_retried_one_character_later() {
    // `\w` before the first start; the search goes on and finds the address after the space.
    assert_finds(find_ip_addresses, "retry", "\u{e9}10.1.2.3 {}", "10.1.2.4");
}

#[test]
fn is_ip_address_takes_a_valid_address_and_refuses_it_with_one_digit_changed() {
    assert!(is_ip_address("192.168.1.20", " "));
    assert!(!is_ip_address("192.168.1.256", " "));
    assert!(is_ip_address("2001:db8::1", " "));
    assert!(!is_ip_address("2001:db8::g", " "));
    assert!(!is_ip_address("", " "));
}

#[test]
fn loopback_and_unspecified_addresses_are_not_personal() {
    for (index, candidate) in [
        "127.0.0.1",
        "127.9.9.9",
        "0.0.0.0",
        "::1",
        "::",
        "0:0:0:0:0:0:0:1",
        "::ffff:127.0.0.1",
        "::ffff:0.0.0.0",
        "::ffff:0:0",
    ]
    .into_iter()
    .enumerate()
    {
        assert!(!is_ip_address(candidate, " "), "{index}");
    }
    assert!(!is_ip_address("::ffff:127.0.0.2", " "));
    assert!(is_ip_address("::ffff:0.0.0.1", " "));
    assert!(is_ip_address("::ffff:192.0.2.128", " "));
}

#[test]
fn an_address_needs_a_digit_the_parser_accepts() {
    assert!(!is_ip_address("abcd::ef", " "));
    assert!(!is_ip_address("a::b", " "));
    assert!(!is_ip_address("::", " "));
    assert!(!is_ip_address("01.2.3.4", " "));
    assert!(!is_ip_address("\u{ff11}.2.3.4", " "));
    assert!(!is_ip_address("1:2:3:4:5:6:7:8:9", " "));
}

#[test]
fn an_ipv4_network_id_is_a_version() {
    assert!(!is_ip_address("10.2.3.0", " "));
    assert!(is_ip_address("10.2.0.3", " "));
    // Only an IPv4 address is a version by its last octet.
    assert!(is_ip_address("::ffff:10.2.3.0", " "));
}

#[test]
fn an_ipv6_slice_is_not_an_address() {
    // At most `start::step`, all decimal; three groups or a hex letter make it an address.
    for (index, candidate) in ["1::2", "10::20", "100::200", "1::", "3::"]
        .iter()
        .enumerate()
    {
        assert!(!is_ip_address(candidate, " "), "slice {index}");
    }
    assert!(is_ip_address("1:2:3::", " "));
    assert!(is_ip_address("1::a", " "));
    assert!(is_ip_address("1::F", " "));
    assert!(is_ip_address("::1:2:3", " "));
}

#[test]
fn an_ipv4_address_after_a_product_or_keyword_is_a_version() {
    let ip = "1.2.3.4";
    for (index, before) in [
        " version ",
        " VERSION  ",
        " Build\t",
        " firmware ",
        " release ",
        " v",
        " v ",
        " ver",
        " ver.",
        " ver. ",
        " Ver ",
        " (v",
        " -v ",
        " Chrome/",
        " x (MyApp/",
        " ;a.b+c-d_e/",
        " ;a\u{e9}/",
        // The product's first letter is `[A-Za-z]` under `(?i)`: the dotted capital I and the
        // dotless i read, as in the reference (checked there on `(\u{130}x/8.8.4.4`).
        " (\u{130}x/",
        " (\u{131}x/",
        // The dotted capital I of Turkish and the dotless i read as `i` under `(?i)`.
        " VERS\u{130}ON ",
        " bu\u{131}ld ",
        " f\u{131}rmware ",
        " a build ",
        // U+0345 is not a word character: a boundary before `v`.
        " \u{345}v",
        "v",
    ]
    .into_iter()
    .enumerate()
    {
        assert!(!is_ip_address(ip, before), "version {index}");
    }
}

#[test]
fn an_ipv4_address_after_something_else_is_an_address() {
    let ip = "1.2.3.4";
    for (index, before) in [
        " ",
        "",
        " at ",
        " Chrome",
        " http://",
        " /srv/",
        " ssh admin@",
        " xv ",
        " \u{e9}v ",
        " 5v ",
        " verb ",
        " versions ",
        " builds ",
        " a\u{345}/",
        " /Chrome/",
        " Chrome/x ",
        " a/b/",
    ]
    .into_iter()
    .enumerate()
    {
        assert!(is_ip_address(ip, before), "address {index}");
    }
}

#[test]
fn only_the_32_characters_before_a_candidate_are_looked_at() {
    // The window holds the text just ahead after a space for the start: 25 spaces and
    // `version` are exactly 32 characters.
    let near = format!("version{}1.2.3.4", " ".repeat(25));
    let far = format!("version{}1.2.3.4", " ".repeat(26));
    assert_misses(find_ip_addresses, "near", &near);
    assert_eq!(find_ip_addresses(&far), vec![(33, 40)]);
}

#[test]
fn the_window_counts_characters_not_bytes() {
    // 25 two-byte spaces (U+00A0 is `\s`) before the candidate are still 25 characters.
    let near = format!("version{}1.2.3.4", "\u{a0}".repeat(25));
    assert_eq!(find_ip_addresses(&near), vec![]);
}

#[test]
fn a_candidate_at_the_start_has_the_start_as_its_window() {
    assert_finds(find_ip_addresses, "start", "{} ok", "192.168.1.20");
}

#[test]
fn an_empty_text_has_no_addresses() {
    assert_misses(find_ip_addresses, "empty", "");
}
