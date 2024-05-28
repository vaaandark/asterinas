// SPDX-License-Identifier: MPL-2.0

use core::borrow::Borrow;

use crate::{
    net::{
        iface::{AnyBoundSocket, AnyUnboundSocket, BindPortConfig, Iface, IpAddress, IpEndpoint},
        IFACES,
    },
    prelude::*, process::current,
};

pub fn get_iface_to_bind(ip_addr: &IpAddress) -> Option<Arc<dyn Iface>> {
    let current = current!();
    let nsproxy = current.nsproxy().lock();
    let net_ns = nsproxy.net_ns().lock();
    let mut ifaces = net_ns.ifaces();
    let IpAddress::Ipv4(ipv4_addr) = ip_addr;
    ifaces
        .find(|iface| {
            if let Some(iface_ipv4_addr) = iface.upgrade().unwrap().ipv4_addr() {
                iface_ipv4_addr == *ipv4_addr
            } else {
                false
            }
        })
        .map(|iface| iface.upgrade().unwrap())
}

/// Get a suitable iface to deal with sendto/connect request if the socket is not bound to an iface.
/// If the remote address is the same as that of some iface, we will use the iface.
/// Otherwise, we will use a default interface.
fn get_ephemeral_iface(remote_ip_addr: &IpAddress) -> Arc<dyn Iface> {
    let current = current!();
    let nsproxy = current.nsproxy().lock();
    let net_ns = nsproxy.net_ns().lock();
    let mut ifaces = net_ns.ifaces();
    let IpAddress::Ipv4(remote_ipv4_addr) = remote_ip_addr;
    if let Some(iface) = ifaces.find(|iface| {
        if let Some(iface_ipv4_addr) = iface.upgrade().unwrap().ipv4_addr() {
            iface_ipv4_addr == *remote_ipv4_addr
        } else {
            false
        }
    }) {
        return iface.upgrade().unwrap();
    }
    // FIXME: use the virtio-net as the default interface
    IFACES.lock().get(0).unwrap().clone()
}

pub(super) fn bind_socket(
    unbound_socket: Box<AnyUnboundSocket>,
    endpoint: &IpEndpoint,
    can_reuse: bool,
) -> core::result::Result<Arc<AnyBoundSocket>, (Error, Box<AnyUnboundSocket>)> {
    let iface = match get_iface_to_bind(&endpoint.addr) {
        Some(iface) => iface,
        None => {
            let err = Error::with_message(Errno::EADDRNOTAVAIL, "Request iface is not available");
            return Err((err, unbound_socket));
        }
    };
    let bind_port_config = match BindPortConfig::new(endpoint.port, can_reuse) {
        Ok(config) => config,
        Err(e) => return Err((e, unbound_socket)),
    };
    iface.bind_socket(unbound_socket, bind_port_config)
}

pub fn get_ephemeral_endpoint(remote_endpoint: &IpEndpoint) -> IpEndpoint {
    let iface = get_ephemeral_iface(&remote_endpoint.addr);
    let ip_addr = iface.ipv4_addr().unwrap();
    IpEndpoint::new(IpAddress::Ipv4(ip_addr), 0)
}
