/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::monitoringserver::NodeInfo;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::str::FromStr;

/// Aggregated information from multiple nodes on the same SoC
#[derive(Debug, Clone)]
pub struct SocInfo {
    pub soc_id: String,
    pub nodes: Vec<NodeInfo>,
    pub total_cpu_usage: f64,
    pub total_cpu_count: u64,
    pub total_gpu_count: u64,
    pub total_used_memory: u64,
    pub total_memory: u64,
    pub total_mem_usage: f64,
    pub total_rx_bytes: u64,
    pub total_tx_bytes: u64,
    pub total_read_bytes: u64,
    pub total_write_bytes: u64,
    pub last_updated: std::time::SystemTime,
}

/// Aggregated information from multiple nodes on the same board
#[derive(Debug, Clone)]
pub struct BoardInfo {
    pub board_id: String,
    pub nodes: Vec<NodeInfo>,
    pub socs: Vec<SocInfo>,
    pub total_cpu_usage: f64,
    pub total_cpu_count: u64,
    pub total_gpu_count: u64,
    pub total_used_memory: u64,
    pub total_memory: u64,
    pub total_mem_usage: f64,
    pub total_rx_bytes: u64,
    pub total_tx_bytes: u64,
    pub total_read_bytes: u64,
    pub total_write_bytes: u64,
    pub last_updated: std::time::SystemTime,
}

/// Data store for managing NodeInfo, SocInfo, and BoardInfo
#[derive(Debug)]
pub struct DataStore {
    pub nodes: HashMap<String, NodeInfo>,
    pub socs: HashMap<String, SocInfo>,
    pub boards: HashMap<String, BoardInfo>,
}

impl DataStore {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            socs: HashMap::new(),
            boards: HashMap::new(),
        }
    }

    /// Stores NodeInfo and updates corresponding SocInfo and BoardInfo
    pub fn store_node_info(&mut self, node_info: NodeInfo) -> Result<(), String> {
        let node_name = node_info.node_name.clone();
        let ip = node_info.ip.clone();

        // Validate IP format
        let _parsed_ip =
            Ipv4Addr::from_str(&ip).map_err(|_| format!("Invalid IP address format: {}", ip))?;

        // Generate IDs based on IP grouping rules
        let soc_id = Self::generate_soc_id(&ip)?;
        let board_id = Self::generate_board_id(&ip)?;

        // Store node and update aggregations
        self.nodes.insert(node_name.clone(), node_info.clone());
        self.update_soc_info(soc_id, node_info.clone())?;
        self.update_board_info(board_id, node_info)?;

        Ok(())
    }

    /// Generates SoC ID: same first 3 octets + same tens place of last octet
    /// Example: 192.168.10.201 and 192.168.10.202 -> same SoC (192.168.10.200)
    pub fn generate_soc_id(ip: &str) -> Result<String, String> {
        let parsed_ip =
            Ipv4Addr::from_str(ip).map_err(|_| format!("Invalid IP address: {}", ip))?;

        let octets = parsed_ip.octets();
        let last_octet = octets[3];
        let soc_group = (last_octet / 10) * 10; // Groups by tens (200-209, 210-219, etc.)

        Ok(format!(
            "{}.{}.{}.{}",
            octets[0], octets[1], octets[2], soc_group
        ))
    }

    /// Generates Board ID: same first 3 octets + same hundreds place of last octet
    /// Example: 192.168.10.201, 192.168.10.202, 192.168.10.222 -> same board (192.168.10.200)
    pub fn generate_board_id(ip: &str) -> Result<String, String> {
        let parsed_ip =
            Ipv4Addr::from_str(ip).map_err(|_| format!("Invalid IP address: {}", ip))?;

        let octets = parsed_ip.octets();
        let last_octet = octets[3];
        let board_group = (last_octet / 100) * 100; // Groups by hundreds (200-299, 300-399, etc.)

        Ok(format!(
            "{}.{}.{}.{}",
            octets[0], octets[1], octets[2], board_group
        ))
    }

    /// Updates or creates SocInfo with the given node
    fn update_soc_info(&mut self, soc_id: String, node_info: NodeInfo) -> Result<(), String> {
        let current_time = std::time::SystemTime::now();

        if let Some(soc_info) = self.socs.get_mut(&soc_id) {
            soc_info.update_with_node(node_info);
            soc_info.last_updated = current_time;
        } else {
            let soc_info = SocInfo::new(soc_id.clone(), node_info);
            self.socs.insert(soc_id, soc_info);
        }

        Ok(())
    }

    /// Updates or creates BoardInfo with the given node
    fn update_board_info(&mut self, board_id: String, node_info: NodeInfo) -> Result<(), String> {
        let current_time = std::time::SystemTime::now();

        if let Some(board_info) = self.boards.get_mut(&board_id) {
            board_info.update_with_node(node_info);
            board_info.last_updated = current_time;
        } else {
            let board_info = BoardInfo::new(board_id.clone(), node_info);
            self.boards.insert(board_id.clone(), board_info);
        }

        // Update SoCs list in BoardInfo
        self.update_board_socs(&board_id)?;

        Ok(())
    }

    /// Updates the SoCs list in a BoardInfo based on current SoCs
    fn update_board_socs(&mut self, board_id: &str) -> Result<(), String> {
        let board_socs: Vec<SocInfo> = self
            .socs
            .values()
            .filter(|soc| {
                if let Ok(soc_board_id) = Self::generate_board_id_from_soc_id(&soc.soc_id) {
                    soc_board_id == board_id
                } else {
                    false
                }
            })
            .cloned()
            .collect();

        // Update the board's SoCs list
        if let Some(board_info) = self.boards.get_mut(board_id) {
            board_info.socs = board_socs;
        }

        Ok(())
    }

    /// Helper function to generate board ID from SoC ID
    fn generate_board_id_from_soc_id(soc_id: &str) -> Result<String, String> {
        // SoC ID format: "192.168.2.200"
        // Board ID format: "192.168.2.200" (same for 200-299 range)
        Self::generate_board_id(soc_id)
    }

    pub fn get_node_info(&self, node_name: &str) -> Option<&NodeInfo> {
        self.nodes.get(node_name)
    }

    pub fn get_soc_info(&self, soc_id: &str) -> Option<&SocInfo> {
        self.socs.get(soc_id)
    }

    pub fn get_board_info(&self, board_id: &str) -> Option<&BoardInfo> {
        self.boards.get(board_id)
    }

    pub fn get_all_nodes(&self) -> &HashMap<String, NodeInfo> {
        &self.nodes
    }

    pub fn get_all_socs(&self) -> &HashMap<String, SocInfo> {
        &self.socs
    }

    pub fn get_all_boards(&self) -> &HashMap<String, BoardInfo> {
        &self.boards
    }
}

impl SocInfo {
    /// Creates new SocInfo with the first node
    pub fn new(soc_id: String, node_info: NodeInfo) -> Self {
        let mut soc_info = Self {
            soc_id,
            nodes: vec![node_info.clone()],
            total_cpu_usage: node_info.cpu_usage,
            total_cpu_count: node_info.cpu_count,
            total_gpu_count: node_info.gpu_count,
            total_used_memory: node_info.used_memory,
            total_memory: node_info.total_memory,
            total_mem_usage: node_info.mem_usage,
            total_rx_bytes: node_info.rx_bytes,
            total_tx_bytes: node_info.tx_bytes,
            total_read_bytes: node_info.read_bytes,
            total_write_bytes: node_info.write_bytes,
            last_updated: std::time::SystemTime::now(),
        };
        soc_info.recalculate_totals();
        soc_info
    }

    /// Updates SocInfo with a new or updated node
    pub fn update_with_node(&mut self, node_info: NodeInfo) {
        // Update existing node or add new one
        if let Some(existing_node) = self
            .nodes
            .iter_mut()
            .find(|n| n.node_name == node_info.node_name)
        {
            *existing_node = node_info.clone();
        } else {
            self.nodes.push(node_info.clone());
        }

        self.recalculate_totals();
    }

    /// Recalculates aggregated values from current nodes
    fn recalculate_totals(&mut self) {
        let node_count = self.nodes.len() as f64;

        if node_count > 0.0 {
            // Average CPU usage percentages across nodes (no per-CPU performance info available)
            self.total_cpu_usage = self.nodes.iter().map(|n| n.cpu_usage).sum::<f64>() / node_count;

            // Calculate memory usage from actual memory values (nodes have different memory sizes)
            self.total_used_memory = self.nodes.iter().map(|n| n.used_memory).sum();
            self.total_memory = self.nodes.iter().map(|n| n.total_memory).sum();
            self.total_mem_usage = if self.total_memory > 0 {
                (self.total_used_memory as f64 * 100.0) / self.total_memory as f64
            } else {
                0.0
            };
        } else {
            self.total_cpu_usage = 0.0;
            self.total_mem_usage = 0.0;
            self.total_used_memory = 0;
            self.total_memory = 0;
        }

        // Sum absolute values across nodes
        self.total_cpu_count = self.nodes.iter().map(|n| n.cpu_count).sum();
        self.total_gpu_count = self.nodes.iter().map(|n| n.gpu_count).sum();
        self.total_rx_bytes = self.nodes.iter().map(|n| n.rx_bytes).sum();
        self.total_tx_bytes = self.nodes.iter().map(|n| n.tx_bytes).sum();
        self.total_read_bytes = self.nodes.iter().map(|n| n.read_bytes).sum();
        self.total_write_bytes = self.nodes.iter().map(|n| n.write_bytes).sum();
    }
}

impl BoardInfo {
    /// Creates new BoardInfo with the first node
    pub fn new(board_id: String, node_info: NodeInfo) -> Self {
        let mut board_info = Self {
            board_id,
            nodes: vec![node_info.clone()],
            socs: Vec::new(), // Populated by update_board_socs
            total_cpu_usage: node_info.cpu_usage,
            total_cpu_count: node_info.cpu_count,
            total_gpu_count: node_info.gpu_count,
            total_used_memory: node_info.used_memory,
            total_memory: node_info.total_memory,
            total_mem_usage: node_info.mem_usage,
            total_rx_bytes: node_info.rx_bytes,
            total_tx_bytes: node_info.tx_bytes,
            total_read_bytes: node_info.read_bytes,
            total_write_bytes: node_info.write_bytes,
            last_updated: std::time::SystemTime::now(),
        };
        board_info.recalculate_totals();
        board_info
    }

    /// Updates BoardInfo with a new or updated node
    pub fn update_with_node(&mut self, node_info: NodeInfo) {
        // Update existing node or add new one
        if let Some(existing_node) = self
            .nodes
            .iter_mut()
            .find(|n| n.node_name == node_info.node_name)
        {
            *existing_node = node_info.clone();
        } else {
            self.nodes.push(node_info.clone());
        }

        self.recalculate_totals();
    }

    /// Recalculates aggregated values from current nodes
    fn recalculate_totals(&mut self) {
        let node_count = self.nodes.len() as f64;

        if node_count > 0.0 {
            // Average CPU usage percentages across nodes (no per-CPU performance info available)
            self.total_cpu_usage = self.nodes.iter().map(|n| n.cpu_usage).sum::<f64>() / node_count;

            // Calculate memory usage from actual memory values (nodes have different memory sizes)
            self.total_used_memory = self.nodes.iter().map(|n| n.used_memory).sum();
            self.total_memory = self.nodes.iter().map(|n| n.total_memory).sum();
            self.total_mem_usage = if self.total_memory > 0 {
                (self.total_used_memory as f64 * 100.0) / self.total_memory as f64
            } else {
                0.0
            };
        } else {
            self.total_cpu_usage = 0.0;
            self.total_mem_usage = 0.0;
            self.total_used_memory = 0;
            self.total_memory = 0;
        }

        // Sum absolute values across nodes
        self.total_cpu_count = self.nodes.iter().map(|n| n.cpu_count).sum();
        self.total_gpu_count = self.nodes.iter().map(|n| n.gpu_count).sum();
        self.total_rx_bytes = self.nodes.iter().map(|n| n.rx_bytes).sum();
        self.total_tx_bytes = self.nodes.iter().map(|n| n.tx_bytes).sum();
        self.total_read_bytes = self.nodes.iter().map(|n| n.read_bytes).sum();
        self.total_write_bytes = self.nodes.iter().map(|n| n.write_bytes).sum();
    }
}
