use std::{
    boxed::Box,
    collections::btree_map::BTreeMap,
    format, println,
    string::{String, ToString},
    vec::Vec,
};

use super::{
    name_objects::{NameSeg, NameString},
    named_objects::{DefDevice, DefMethod, DefPowerRes, DefProcessor, DefThermalZone},
    namespace_modifier::DefScope,
};

pub struct Namespace {
    pub current_namespace: Vec<Box<[NameSeg]>>,
    pub root: NamespaceNode,
}

#[derive(Debug, Default)]
pub struct NamespaceNode {
    //type could be an enum of Scope, Device, PowerRes, ThermalZone, Processor
    pub methods: BTreeMap<NameSeg, (usize, Option<DefMethod>)>,
    pub children: BTreeMap<NameSeg, NamespaceNode>,
}

impl NamespaceNode {
    pub fn search_node_mut(&mut self, name: &[NameSeg]) -> Option<&mut NamespaceNode> {
        let mut current_node = self;
        for seg in name {
            if let Some(node) = current_node.children.get_mut(seg) {
                current_node = node;
            } else {
                return None;
            }
        }
        Some(current_node)
    }

    pub fn search_node(&self, name: &[NameSeg]) -> Option<&NamespaceNode> {
        let mut current_node = self;
        for seg in name {
            if let Some(node) = current_node.children.get(seg) {
                current_node = node;
            } else {
                return None;
            }
        }
        Some(current_node)
    }

    pub fn method_exists(&self, name: &NameSeg) -> bool {
        self.methods.contains_key(name)
    }

    pub fn methods_have_bodies(&self, namespace_str: String) -> String {
        let mut message = "".to_string();
        for (name, (_, method)) in self.methods.iter() {
            if method.is_none() {
                message += format!("Method {:?} has no body in namespace {}\n", name, namespace_str).as_str();
            }
        }
        for (scope_name, node) in self.children.iter() {
            let scope_name_string: String = scope_name.into();
            let name = node.methods_have_bodies(namespace_str.clone() + &scope_name_string);
            message += &name;
        }
        message
    }
}
static mut GLOBAL_NAMESPACE: Option<Namespace> = None;

pub fn create_namespace() {
    unsafe {
        let mut namespace = Namespace {
            current_namespace: Vec::new(),
            root: NamespaceNode::default(),
        };
        namespace.root.children.insert("_GPE".into(), NamespaceNode::default());
        namespace.root.children.insert("_PR_".into(), NamespaceNode::default());
        namespace.root.children.insert("_SB_".into(), NamespaceNode::default());
        namespace.root.children.insert("_SI_".into(), NamespaceNode::default());
        namespace.root.children.insert("_TZ_".into(), NamespaceNode::default());
        GLOBAL_NAMESPACE = Some(namespace);
    }
}

pub fn get_namespace() -> &'static mut Namespace {
    unsafe {
        match &mut GLOBAL_NAMESPACE {
            Some(namespace) => namespace,
            None => panic!("Namespace not created"),
        }
    }
}

impl Namespace {
    pub fn add_method(&mut self, name: &NameString, method: DefMethod) {
        let (node, last_seg) = self.get_node_for_method(name).unwrap();
        if !node.method_exists(last_seg) {
            panic!("Method with name {:#?} not found in namespace", name);
        }
        node.methods
            .insert(*last_seg, (method.method_flags.get_arg_count() as usize, Some(method)));
    }

    pub fn add_method_arg_count(&mut self, name: &NameString, arg_count: usize) {
        let (node, last_seg) = self
            .get_node_for_method(name)
            .expect(format!("Method with name {:#?} not found in namespace", name).as_str());
        if node.method_exists(last_seg) {
            panic!("Method with name {:#?} already exists in namespace", name);
        }
        node.methods.insert(*last_seg, (arg_count, None));
    }

    fn get_node_for_method<'a>(&'a mut self, name: &'a NameString) -> Option<(&'a mut NamespaceNode, &'a NameSeg)> {
        match &name {
            NameString::Rootchar(name_path) => {
                let seg_slice: &[NameSeg] = name_path.into();
                Some((
                    self.root.search_node_mut(&seg_slice[..(seg_slice.len() - 1)]).unwrap(),
                    seg_slice.last().unwrap(),
                ))
            }
            NameString::PrefixPath(prefix_path) => {
                let current_path = self.get_namespace_sequence();
                let node = self
                    .root
                    .search_node(&current_path[..(current_path.len() - prefix_path.0 as usize)])?;

                #[allow(invalid_reference_casting)]
                let node = unsafe { &mut *(node as *const NamespaceNode as *mut NamespaceNode) };

                let seg_slice: &[NameSeg] = (&prefix_path.1).into();
                Some((
                    node.search_node_mut(&seg_slice[..(seg_slice.len() - 1)]).unwrap(),
                    seg_slice.last().unwrap(),
                ))
            }
            NameString::BlankPath(name_path) => {
                let current_path = self.get_namespace_sequence();
                let node = self.root.search_node(current_path)?;

                #[allow(invalid_reference_casting)]
                let node = unsafe { &mut *(node as *const NamespaceNode as *mut NamespaceNode) };

                let seg_slice: &[NameSeg] = (name_path).into();
                if seg_slice.is_empty() {
                    panic!("Cannot add method with empty name");
                }
                Some((
                    node.search_node_mut(&seg_slice[..(seg_slice.len() - 1)]).unwrap(),
                    seg_slice.last().unwrap(),
                ))
            }
        }
    }

    pub fn get_method(&self, name: &NameString) -> Option<&(usize, Option<DefMethod>)> {
        match &name {
            NameString::Rootchar(name_path) => {
                let seg_slice: &[NameSeg] = name_path.into();
                self.root
                    .search_node(&seg_slice[..(seg_slice.len() - 1)])?
                    .methods
                    .get(seg_slice.last()?)
            }
            NameString::PrefixPath(prefix_path) => {
                let seg_slice: &[NameSeg] = (&prefix_path.1).into();
                let current_path = self.get_namespace_sequence();
                let node = self
                    .root
                    .search_node(&current_path[..(current_path.len() - prefix_path.0 as usize)])?;
                node.search_node(&seg_slice[..(seg_slice.len() - 1)])?
                    .methods
                    .get(seg_slice.last()?)
            }
            NameString::BlankPath(name_path) => {
                let nodes = self.get_current_namespace_nodes()?;
                //iterate over nodes from last to first and check
                let segments: &[NameSeg] = name_path.into();
                for node in nodes.iter().rev() {
                    let inner_node = node.search_node(&segments[..segments.len() - 1])?;
                    if let Some(method) = inner_node.methods.get(segments.last()?) {
                        return Some(method);
                    }
                }
                None
            }
        }
    }

    pub fn print_methods(&self) {
        //for (name, (arg_count, _)) in self.methods.iter() {
        //    println!("Method: {}, Arg count: {}", name, arg_count);
        //    std::thread::sleep(std::time::Duration::from_secs(1));
        //}
        println!("not implemented");
    }

    pub fn get_namespace_sequence(&self) -> &[NameSeg] {
        self.current_namespace.last().map_or(&[], |old| old)
    }

    pub fn get_current_namespace_nodes(&self) -> Option<Vec<&NamespaceNode>> {
        let mut acc = Vec::new();
        let mut current_node = &self.root;
        acc.push(current_node);
        let segments = self.get_namespace_sequence();
        for segment in segments {
            if let Some(node) = current_node.children.get(segment) {
                current_node = node;
                acc.push(current_node);
            } else {
                return None;
            }
        }
        Some(acc)
    }

    ///SAFETY: Using a namespace node to remove its children, then accessing the child through the
    ///vec is invalid. Only add nodes
    pub unsafe fn get_current_namespace_nodes_mut(&mut self) -> Option<Vec<&mut NamespaceNode>> {
        let mut acc = Vec::new();
        unsafe { acc.push(&mut *(&mut self.root as *mut NamespaceNode)) };
        let segments = unsafe { &*(self.get_namespace_sequence() as *const [NameSeg]) };
        let mut current_node = &mut self.root;
        for segment in segments {
            if let Some(node) = current_node.children.get_mut(segment) {
                current_node = node;
                unsafe { acc.push(&mut *(current_node as *mut NamespaceNode)) };
            } else {
                return None;
            }
        }
        Some(acc)
    }

    pub fn push_namespace_segment(&mut self, name: NameSeg) {
        let old_namespace: &[NameSeg] = self.current_namespace.last().map_or(&[], |old| old);
        let mut new_namespace = Vec::new();
        new_namespace.extend(old_namespace);
        new_namespace.push(name);
        self.current_namespace.push(new_namespace.into());
    }

    //doesn't work if name is more than 1 segment long
    pub fn push_namespace_string(&mut self, name: NameString) {
        match &name {
            NameString::Rootchar(name_path) => {
                let new_namespace: Box<[NameSeg]> = name_path.clone().into();
                self.current_namespace.push(new_namespace);
            }
            NameString::PrefixPath(name_path) => {
                let old_namespace: &[NameSeg] = self.current_namespace.last().map_or(&[], |old| old);
                let new_segments: &[NameSeg] = (&name_path.1).into();
                let mut new_namespace = Vec::new();

                new_namespace.extend(&old_namespace[..old_namespace.len() - name_path.0 as usize]);
                new_namespace.extend(new_segments);
                self.current_namespace.push(new_namespace.into());
            }
            NameString::BlankPath(name_path) => {
                let old_namespace: &[NameSeg] = self.current_namespace.last().map_or(&[], |old| old);
                let segments: &[NameSeg] = name_path.into();
                let mut new_namespace = Vec::new();

                new_namespace.extend(old_namespace);
                new_namespace.extend(segments);
                self.current_namespace.push(new_namespace.into());
            }
        };

        let segments = unsafe { &*(self.get_namespace_sequence() as *const [NameSeg]) };
        if segments.is_empty() {
            return;
        }

        //everything before must exist already
        let node = self
            .root
            .search_node_mut(&segments[..(segments.len() - 1)])
            .expect(&format!("cannot extend namespace, name path was {:?}", segments));
        let last_segment = *segments.last().unwrap();
        if node.children.contains_key(&last_segment) {
            return;
        }
        node.children.insert(
            last_segment,
            NamespaceNode {
                methods: BTreeMap::new(),
                children: BTreeMap::new(),
            },
        );
    }

    pub fn pop_namespace(&mut self) {
        self.current_namespace.pop();
    }

    pub fn scan_for_methods(&mut self, byte_stream: &[u8]) {
        let mut i = 0;
        while i < byte_stream.len() {
            if let Some((name, arg_count, pkg_len)) = DefMethod::get_arg_count(&byte_stream[i..]) {
                self.add_method_arg_count(&name, arg_count as usize);

                i += pkg_len;
                if i > byte_stream.len() {
                    panic!("Namespace scan failed. i: {}, byte_stream.len(): {}", i, byte_stream.len());
                }
            } else if let Some((name, skip)) = self.cehck_namespace_change(&byte_stream[i..]) {
                let scope_end = i + skip;
                let scope_data = &byte_stream[i..scope_end];
                self.push_namespace_string(name);
                self.scan_for_methods(&scope_data[1..]);
                self.pop_namespace();
                i += skip;
            } else {
                i += 1;
            }
        }
    }

    fn cehck_namespace_change(&self, byte_stream: &[u8]) -> Option<(NameString, usize)> {
        if let Some(res) = DefScope::check_namespace(byte_stream) {
            return Some(res);
        }
        if let Some(res) = DefDevice::check_namespace(byte_stream) {
            return Some(res);
        }
        if let Some(res) = DefPowerRes::check_namespace(byte_stream) {
            return Some(res);
        }
        if let Some(res) = DefThermalZone::check_namespace(byte_stream) {
            return Some(res);
        }
        if let Some(res) = DefProcessor::check_namespace(byte_stream) {
            return Some(res);
        }
        None
    }
}
