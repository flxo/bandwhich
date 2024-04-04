use std::{collections::HashMap, net};

use procfs::net::{TcpNetEntry, UdpNetEntry};

use crate::{
    network::{LocalSocket, Protocol},
    os::ProcessInfo,
    OpenSockets,
};

pub(crate) fn get_open_sockets() -> OpenSockets {
    let Ok(processes) = procfs::process::all_processes() else {
        return OpenSockets {
            sockets_to_procs: HashMap::new(),
        };
    };

    let sockets_to_procs = processes
        .filter_map(Result::ok)
        .filter_map(|process| {
            // Collect the network entries from the process specific table
            let tcp_entries = process
                .tcp()
                .unwrap_or_default()
                .into_iter()
                .chain(process.tcp6().unwrap_or_default())
                .map(Entry::Tcp);
            let udp_entries = process
                .udp()
                .unwrap_or_default()
                .into_iter()
                .chain(process.udp6().unwrap_or_default())
                .map(Entry::Udp);

            let entries = tcp_entries.chain(udp_entries);

            process.stat().ok().map(|stat| {
                entries.map(move |entry| {
                    let proc_info = ProcessInfo::new(&stat.comm, stat.pid as u32);
                    let socket = LocalSocket {
                        ip: entry.local_address().ip(),
                        port: entry.local_address().port(),
                        protocol: entry.protocol(),
                    };
                    (socket, proc_info)
                })
            })
        })
        .flatten()
        .collect();

    OpenSockets { sockets_to_procs }
}

/// Helper to treap Udp and Tcp entries as one type
enum Entry {
    Tcp(TcpNetEntry),
    Udp(UdpNetEntry),
}

impl Entry {
    fn local_address(&self) -> net::SocketAddr {
        match self {
            Entry::Tcp(entry) => entry.local_address,
            Entry::Udp(entry) => entry.local_address,
        }
    }

    fn protocol(&self) -> Protocol {
        match self {
            Entry::Tcp(_) => Protocol::Tcp,
            Entry::Udp(_) => Protocol::Udp,
        }
    }
}
