use std::error::Error;
use std::net::ToSocketAddrs;
use std::collections::HashMap;

use rmididings::proc::*;
use rmididings::hook::Hook;
use rmididings::util::OSCServer;

extern crate rosc;
use rosc::{OscMessage, OscType as o};

// Carla callback opcodes - https://github.com/falkTX/Carla/blob/2a6a7de04f75daf242ae9d8c99b349ea7dc6ff7f/source/backend/CarlaBackend.h
const ENGINE_CALLBACK_PLUGIN_REMOVED: i32 = 2;
const ENGINE_CALLBACK_PARAMETER_VALUE_CHANGED: i32 = 5;

pub trait CarlaPluginHandler<'a> {
    fn get_plugin_urls(&self) -> Vec<&'static str>;

    fn set_send_value_fn(&mut self, f: &'a fn(i32, i32, f32)) {}

    fn on_plugin_added(&mut self, id: i32, url: &str) -> Result<Option<Box<dyn FilterTrait>>, Box<dyn Error>> { Ok(None) }

    fn on_plugin_removed(&mut self, id: i32) -> Result<Option<Box<dyn FilterTrait>>, Box<dyn Error>> { Ok(None) }

    fn on_param_changed(&mut self, id: i32, param: i32, value: f32) -> Result<Option<Box<dyn FilterTrait>>, Box<dyn Error>> { Ok(None) }

    fn on_value_changed(&mut self, id: i32, param: i32, value: f32) -> Result<Option<Box<dyn FilterTrait>>, Box<dyn Error>> { Ok(None) }
}

pub struct CarlaOSC<'a> {
    server: OSCServer,
    listen_ip: Option<String>,
    registered_udp: bool,
    registered_tcp: bool,
    plugin_urls: HashMap<&'a str, Vec<usize>>,
    plugin_ids: HashMap<i32, Vec<usize>>,
    plugin_handlers: Vec<Box<dyn CarlaPluginHandler<'a> + 'a>>,
}

/// OSC Interface to Carla
impl<'a> CarlaOSC<'a> {
    pub fn new<T: ToSocketAddrs, U: ToSocketAddrs>(listen_addr: T, carla_addr: U) -> Self {
        let mut server = OSCServer::new();
        server.listen_udp(&listen_addr);
        server.notify_udp(&carla_addr);
        server.listen_tcp(&listen_addr);
        server.connect_tcp(&carla_addr);

        let mut listen_ip = None;
        if let Ok(mut iter) = listen_addr.to_socket_addrs() {
            if let Some(addr) = iter.next() {
                listen_ip = Some(addr.ip().to_string());
            }
        }
 
        Self {
            server,
            listen_ip,
            registered_udp: false,
            registered_tcp: false,
            plugin_ids: HashMap::new(),
            plugin_urls: HashMap::new(),
            plugin_handlers: vec![],
        }
    }

    pub fn default() -> Self {
        Self::new("localhost:22753", "localhost:22752")
    }

    pub fn with<T: CarlaPluginHandler<'a> + 'a>(mut self, plugin_handler: T) -> Self {
        let plugin_idx = self.plugin_handlers.len();
        for url in plugin_handler.get_plugin_urls().iter() {
            if let Some(v) = self.plugin_urls.get_mut(url) {
                v.push(plugin_idx);
            } else {
                self.plugin_urls.insert(url, vec![plugin_idx]);
            }
        }
        self.plugin_handlers.push(Box::new(plugin_handler));
        self
    }

    fn on_osc_message(&mut self, message: OscMessage) -> Result<Option<Box<dyn FilterTrait>>, Box<dyn Error>> {
        // if message.addr.as_str() != "/Carla/runtime" { println!("on_osc_message {:?}", message); }
        match (message.addr.as_str(), message.args.as_slice()) {
            ("/Carla/info", [
                    o::Int(id), o::Int(_), o::Int(_), o::Int(_), o::Long(_), o::Int(_), o::Int(_), o::String(_),
                    o::String(_), o::String(_), o::String(_), o::String(url), o::String(_), o::String(_)]) => {
                self.on_plugin_added(*id, &url.as_str())
            },
            ("/Carla/param", [o::Int(id), o::Int(param), o::Float(value)]) => {
                self.on_param_changed(*id, *param, *value)
            },
            ("/Carla/cb", [o::Int(action), o::Int(id), o::Int(ival), o::Int(_), o::Int(_), o::Float(fval), o::String(_)]) => {
                match *action {
                    ENGINE_CALLBACK_PARAMETER_VALUE_CHANGED => self.on_value_changed(*id, *ival, *fval),
                    ENGINE_BALLBACK_ENGINE_REMOVED => self.on_plugin_removed(*id),
                    _ => Ok(None),
                }
            },
            ("/Carla/paramData", [o::Int(id), o::Int(ival), o::Int(_), o::Int(_), o::Int(_), o::Int(_), o::Float(_), o::Float(_), o::Float(fval)]) => {
                self.on_value_changed(*id, *ival, *fval)
            }
           _ => Ok(None),
        }
    }

    fn register(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(url) = self.server.get_osc_listen_url_tcp() {
            if self.server.send_osc_tcp("/register", vec![o::String(format!("{}/Carla", url))])? > 0 {
                self.registered_tcp = true;
            }
        }
        if let Some(url) = self.server.get_osc_listen_url_udp() {
            if self.server.send_osc_udp("/register", vec![o::String(format!("{}/Carla", url))])? > 0 {
                self.registered_udp = true;
            }
        }
        Ok(())
    }

    fn unregister(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(listen_ip) = &self.listen_ip {
            // Carla requires unregistering with just the IP address.
            if self.registered_tcp {
                if self.server.send_osc_tcp("/unregister", vec![o::String(listen_ip.to_string())])? > 0 {
                    self.registered_tcp = false;
                }
            }
            if self.registered_udp {
                if self.server.send_osc_udp("/unregister", vec![o::String(listen_ip.to_string())])? > 0 {
                    self.registered_udp = false;
                }
            }
        }
        Ok(())
    }

    fn on_plugin_added(&mut self, id: i32, url: &str) -> Result<Option<Box<dyn FilterTrait>>, Box<dyn Error>> {
        if let Some(handler_idxs) = self.plugin_urls.get(url) {
            let mut filters: Vec<Box<dyn FilterTrait>> = vec![];
            for handler_idx in handler_idxs {
                // Store plugin id.
                if let Some(v) = self.plugin_ids.get_mut(&id) {
                    v.push(*handler_idx);
                } else {
                    self.plugin_ids.insert(id, vec![*handler_idx]);
                }
                // Run handler.
                if let Some(handler) = self.plugin_handlers.get_mut(*handler_idx) {
                    if let Some(result) = handler.on_plugin_added(id, url)? {
                        filters.push(result);
                    }
                }
            }
            Ok(as_filter_chain(filters))
        } else {
            Ok(None)
        }
    }

    fn on_plugin_removed(&mut self, id: i32) -> Result<Option<Box<dyn FilterTrait>>, Box<dyn Error>> {
        if let Some(handlers) = self.plugin_ids.get_mut(&id) {
            let mut filters: Vec<Box<dyn FilterTrait>> = vec![];
            for handler_idx in handlers.iter_mut() {
                if let Some(handler) = self.plugin_handlers.get_mut(*handler_idx) {
                    if let Some(result) = handler.on_plugin_removed(id)? {
                        filters.push(result);
                    }
                }
            }
            Ok(as_filter_chain(filters))
        } else {
            Ok(None)
        }
    }

    fn on_value_changed(&mut self, id: i32, param: i32, value: f32) -> Result<Option<Box<dyn FilterTrait>>, Box<dyn Error>> {
        if let Some(handlers) = self.plugin_ids.get_mut(&id) {
            // println!("Carla value changed for id {}: {} = {}", id, param, value);
            let mut filters: Vec<Box<dyn FilterTrait>> = vec![];
            for handler_idx in handlers.iter_mut() {
                if let Some(handler) = self.plugin_handlers.get_mut(*handler_idx) {
                    if let Some(result) = handler.on_value_changed(id, param, value)? {
                        filters.push(result);
                    }
                }
            }
            Ok(as_filter_chain(filters))
        } else {
            Ok(None)
        }
    }

    fn on_param_changed(&mut self, id: i32, param: i32, value: f32) -> Result<Option<Box<dyn FilterTrait>>, Box<dyn Error>> {
        if let Some(handlers) = self.plugin_ids.get_mut(&id) {
            // println!("Carla param changed for id {}: {} = {}", id, param, value);
            let mut filters: Vec<Box<dyn FilterTrait>> = vec![];
            for handler_idx in handlers.iter_mut() {
                if let Some(handler) = self.plugin_handlers.get_mut(*handler_idx) {
                    if let Some(result) = handler.on_param_changed(id, param, value)? {
                        filters.push(result);
                    }
                }
            }
            Ok(as_filter_chain(filters))
        } else {
            Ok(None)
        }
    }
}

impl Hook for CarlaOSC<'_> {
    fn on_start(&mut self) -> Result<Option<Box<dyn FilterTrait>>, Box<dyn Error>> {
        self.server.start()?;
        self.register()?;
        Ok(None)
    }

    fn on_exit(&mut self) -> Result<Option<Box<dyn FilterTrait>>, Box<dyn Error>> {
        self.unregister()?;
        self.server.stop()?;
        Ok(None)
    }

    fn get_pollfds(&mut self) -> Result<Vec<i32>, Box<dyn Error>> {
        self.server.get_pollfds()
    }

    fn run(&mut self) -> Result<Option<Box<dyn FilterTrait>>, Box<dyn Error>> {
        let mut filters = Vec::<Box<dyn FilterTrait>>::new();

        while let Some(message) = self.server.run()? {
            if let Some(filter) = self.on_osc_message(message)? {
                filters.push(filter);
            }
        }

        if filters.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Box::new(FilterChain::new(ConnectionType::Chain, filters))))
        }
    }
}

fn as_filter_chain(mut filters: Vec<Box<dyn FilterTrait>>) -> Option<Box<dyn FilterTrait>> {
    if filters.is_empty() {
        return None;
    }

    if filters.len() == 1 {
        if let Some(filter) = filters.pop() {
            return Some(filter);
        }
        // should be unreachable code
    }

    return Some(Box::new(FilterChain::new(ConnectionType::Chain, filters)));
}