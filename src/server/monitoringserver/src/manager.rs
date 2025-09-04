//! MonitoringServerManager: Asynchronous manager for MonitoringServer
//!
//! This struct manages scenario requests received via gRPC, and provides
//! a gRPC sender for communicating with the nodeagent or other services.
//! It is designed to be thread-safe and run in an async context.
use crate::data_structures::{DataStore, SocInfo, BoardInfo};
use common::monitoringserver::{ContainerList, NodeInfo};
use common::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use std::net::Ipv4Addr;
use std::str::FromStr;

/// Main manager struct for MonitoringServer.
///
/// Holds the gRPC receiver and sender, and manages the main event loop.
pub struct MonitoringServerManager {
    /// Receiver for container information from gRPC
    rx_container: Arc<Mutex<mpsc::Receiver<ContainerList>>>,
    /// Receiver for node information from gRPC
    rx_node: Arc<Mutex<mpsc::Receiver<NodeInfo>>>,
    /// Data store for managing NodeInfo, SocInfo, and BoardInfo
    data_store: Arc<Mutex<DataStore>>,
}

impl MonitoringServerManager {
    /// Creates a new MonitoringServerManager instance.
    ///
    /// # Arguments
    /// * `rx_container` - Channel receiver for container information
    /// * `rx_node` - Channel receiver for node information
    pub async fn new(
        rx_container: mpsc::Receiver<ContainerList>,
        rx_node: mpsc::Receiver<NodeInfo>,
    ) -> Self {
        Self {
            rx_container: Arc::new(Mutex::new(rx_container)),
            rx_node: Arc::new(Mutex::new(rx_node)),
            data_store: Arc::new(Mutex::new(DataStore::new())),
        }
    }

    /// Initializes the MonitoringServerManager (e.g., loads scenarios, prepares state).
    pub async fn initialize(&mut self) -> Result<()> {
        println!("MonitoringServerManager init");
        // Add initialization logic here (e.g., read scenarios, subscribe, etc.)
        Ok(())
    }

    /// Processes ContainerList messages from nodeagent.
    ///
    /// This function handles the received ContainerList and processes it accordingly.
    async fn handle_container_list(&self, container_list: ContainerList) {
        println!(
            "[MonitoringServer] Received ContainerList from {}: containers count={}",
            container_list.node_name, 
            container_list.containers.len()
        );
        
        // Print container details
        for container in &container_list.containers {
            println!(
                "  Container: ID={}, Names={:?}, Image={}",
                container.id, container.names, container.image
            );
        }
        
        // TODO: Add your container processing logic here
    }

    /// Processes NodeInfo messages from nodeagent.
    ///
    /// This function handles the received NodeInfo and processes it accordingly.
    async fn handle_node_info(&self, node_info: NodeInfo) {
        // Print detailed NodeInfo first
        self.print_node_info(&node_info);
        
        // Store NodeInfo and update SocInfo/BoardInfo
        {
            let mut data_store = self.data_store.lock().await;
            match data_store.store_node_info(node_info.clone()) {
                Ok(_) => {
                    println!("[MonitoringServer] ✅ Successfully stored NodeInfo for {}", node_info.node_name);

                    // **ENHANCED**: Print ID generation details
                    self.print_id_generation_details(&node_info.ip);

                    // Print aggregated information
                    self.print_aggregated_info(&data_store, &node_info.ip).await;

                    // **ENHANCED**: Print detailed SoC mapping
                    self.print_detailed_soc_mapping(&data_store).await;

                    // Print summary statistics
                    self.print_summary_stats(&data_store).await;
                }
                Err(e) => {
                    eprintln!("[MonitoringServer] ❌ Error storing NodeInfo: {}", e);
                }
            }
        }
        
        println!("{}", "=".repeat(80)); // Separator line
    }

    /// Print ID generation details for debugging
    fn print_id_generation_details(&self, ip: &str) {
        println!("\n🔍 ID GENERATION DEBUG");
        println!("┌─────────────────────────────────────────────────────────────────────────────┐");
        println!("│ Input IP: {:<66} │", ip);
        
        if let Ok(soc_id) = DataStore::generate_soc_id(ip) {
            println!("│ Generated SoC ID: {:<58} │", soc_id);
        }
        
        if let Ok(board_id) = DataStore::generate_board_id(ip) {
            println!("│ Generated Board ID: {:<56} │", board_id);
        }
        
        // Show the logic
        if let Ok(parsed_ip) = std::net::Ipv4Addr::from_str(ip) {
            let octets = parsed_ip.octets();
            let last_octet = octets[3];
            let soc_group = (last_octet / 10) * 10;
            let board_group = (last_octet / 100) * 100;
            
            println!("│ Last Octet: {:<3} → SoC Group: {:<3} → Board Group: {:<3}              │", 
                     last_octet, soc_group, board_group);
        }
        println!("└─────────────────────────────────────────────────────────────────────────────┘");
    }

    /// Print detailed SoC mapping for all current data
    async fn print_detailed_soc_mapping(&self, data_store: &DataStore) {
        println!("\n🗺️  DETAILED SOC MAPPING");
        println!("┌─────────────────────────────────────────────────────────────────────────────┐");
        
        for (soc_id, soc_info) in data_store.get_all_socs() {
            println!("│ SoC: {:<20} │ Nodes: {:<2} │ Nodes List: {:<25} │", 
                     soc_id, 
                     soc_info.nodes.len(),
                     soc_info.nodes.iter()
                         .map(|n| n.node_name.clone())
                         .collect::<Vec<_>>()
                         .join(", "));
        }
        
        println!("├─────────────────────────────────────────────────────────────────────────────┤");
        
        for (board_id, board_info) in data_store.get_all_boards() {
            println!("│ Board: {:<18} │ Nodes: {:<2} │ SoCs: {:<2} │ SoC List: {:<15} │", 
                     board_id, 
                     board_info.nodes.len(),
                     board_info.socs.len(),
                     board_info.socs.iter()
                         .map(|s| s.soc_id.split('.').last().unwrap_or(""))
                         .collect::<Vec<_>>()
                         .join(", "));
        }
        println!("└─────────────────────────────────────────────────────────────────────────────┘");
    }

    /// Enhanced Board info printing with SoC details
    fn print_board_info(&self, board_info: &BoardInfo) {
        println!("\n🖥️  BOARD INFORMATION");
        println!("┌─────────────────────────────────────────────────────────────────────────────┐");
        println!("│ Board ID: {:<66} │", board_info.board_id);
        println!("│ Nodes Count: {:<6} │ SoCs Count: {:<6} │ Updated: {:<20} │", 
                 board_info.nodes.len(), 
                 board_info.socs.len(),
                 self.format_time_ago(&board_info.last_updated));
        
        // **NEW**: Show SoCs in this board
        if !board_info.socs.is_empty() {
            println!("├─────────────────────────────────────────────────────────────────────────────┤");
            println!("│ SoCs in this Board:                                                        │");
            for (i, soc) in board_info.socs.iter().enumerate() {
                println!("│  {}. SoC: {:<25} │ Nodes: {:<2} │ Avg CPU: {:<6.2}%        │", 
                         i + 1, soc.soc_id, soc.nodes.len(), soc.total_cpu_usage);
            }
        }
        
        println!("├─────────────────────────────────────────────────────────────────────────────┤");
        println!("│ Board-wide Aggregated Metrics:                                             │");
        println!("│   CPU: {:<7.2}% │ Total Cores: {:<8} │ GPU Units: {:<8} │ Efficiency: {:<6} │", 
                 board_info.total_cpu_usage, 
                 board_info.total_cpu_count, 
                 board_info.total_gpu_count,
                 self.calculate_efficiency(board_info.total_cpu_usage));
        println!("│   Memory: {:<4.2}% │ Used: {:<11} │ Total: {:<11} │ Free: {:<8} │", 
                 board_info.total_mem_usage,
                 self.format_memory(board_info.total_used_memory),
                 self.format_memory(board_info.total_memory),
                 self.format_memory(board_info.total_memory - board_info.total_used_memory));
        println!("├─────────────────────────────────────────────────────────────────────────────┤");
        println!("│ Nodes on this Board (grouped by SoC):                                     │");
        for (i, node) in board_info.nodes.iter().enumerate() {
            let status = if node.cpu_usage > 80.0 { "🔴 HIGH" } 
                        else if node.cpu_usage > 50.0 { "🟡 MED" } 
                        else { "🟢 LOW" };
            // Show which SoC this node belongs to
            let soc_id = DataStore::generate_soc_id(&node.ip).unwrap_or_default();
            println!("│  {}. {:<25} │ SoC: {:<15} │ CPU: {:<6.2}% {} │", 
                     i + 1, node.node_name, soc_id, node.cpu_usage, status);
        }
        println!("└─────────────────────────────────────────────────────────────────────────────┘");
    }

    /// Prints detailed NodeInfo in a formatted way
    fn print_node_info(&self, node_info: &NodeInfo) {
        println!("\n📊 NODE INFORMATION");
        println!("┌─────────────────────────────────────────────────────────────────────────────┐");
        println!("│ Node: {:<70} │", node_info.node_name);
        println!("│ IP Address: {:<64} │", node_info.ip);
        println!("├─────────────────────────────────────────────────────────────────────────────┤");
        println!("│ CPU Usage: {:<6.2}% │ Cores: {:<6} │ GPU Units: {:<6} │ OS: {:<15} │", 
                 node_info.cpu_usage, node_info.cpu_count, node_info.gpu_count, node_info.os);
        println!("│ Memory: {:<7.2}% │ Used: {:<8} KB │ Total: {:<8} KB │ Arch: {:<12} │", 
                 node_info.mem_usage, 
                 self.format_memory(node_info.used_memory), 
                 self.format_memory(node_info.total_memory), 
                 node_info.arch);
        println!("├─────────────────────────────────────────────────────────────────────────────┤");
        println!("│ Network - RX: {:<15} │ TX: {:<15} │ Total: {:<15} │", 
                 self.format_bytes(node_info.rx_bytes), 
                 self.format_bytes(node_info.tx_bytes),
                 self.format_bytes(node_info.rx_bytes + node_info.tx_bytes));
        println!("│ Disk I/O - Read: {:<12} │ Write: {:<12} │ Total: {:<12} │", 
                 self.format_bytes(node_info.read_bytes), 
                 self.format_bytes(node_info.write_bytes),
                 self.format_bytes(node_info.read_bytes + node_info.write_bytes));
        println!("└─────────────────────────────────────────────────────────────────────────────┘");
    }

    /// Prints aggregated SoC and Board information
    async fn print_aggregated_info(&self, data_store: &DataStore, ip: &str) {
        // Print SoC info
        if let Ok(soc_id) = DataStore::generate_soc_id(ip) {
            if let Some(soc_info) = data_store.get_soc_info(&soc_id) {
                self.print_soc_info(soc_info);
            }
        }

        // Print Board info
        if let Ok(board_id) = DataStore::generate_board_id(ip) {
            if let Some(board_info) = data_store.get_board_info(&board_id) {
                self.print_board_info(board_info);
            }
        }
    }

    /// Prints detailed SoC information
    fn print_soc_info(&self, soc_info: &SocInfo) {
        println!("\n🔧 SOC INFORMATION");
        println!("┌─────────────────────────────────────────────────────────────────────────────┐");
        println!("│ SoC ID: {:<68} │", soc_info.soc_id);
        println!("│ Nodes Count: {:<63} │", soc_info.nodes.len());
        println!("├─────────────────────────────────────────────────────────────────────────────┤");
        println!("│ Aggregated Metrics:                                                        │");
        println!("│   CPU: {:<7.2}% │ Total Cores: {:<8} │ GPU Units: {:<8} │ Updated: {:<8} │", 
                 soc_info.total_cpu_usage, 
                 soc_info.total_cpu_count, 
                 soc_info.total_gpu_count,
                 self.format_time_ago(&soc_info.last_updated));
        println!("│   Memory: {:<4.2}% │ Used: {:<11} │ Total: {:<11} │ Free: {:<8} │", 
                 soc_info.total_mem_usage,
                 self.format_memory(soc_info.total_used_memory),
                 self.format_memory(soc_info.total_memory),
                 self.format_memory(soc_info.total_memory - soc_info.total_used_memory));
        println!("│   Network: RX {:<12} │ TX {:<12} │ Total {:<12} │", 
                 self.format_bytes(soc_info.total_rx_bytes),
                 self.format_bytes(soc_info.total_tx_bytes),
                 self.format_bytes(soc_info.total_rx_bytes + soc_info.total_tx_bytes));
        println!("│   Disk I/O: Read {:<9} │ Write {:<9} │ Total {:<9} │", 
                 self.format_bytes(soc_info.total_read_bytes),
                 self.format_bytes(soc_info.total_write_bytes),
                 self.format_bytes(soc_info.total_read_bytes + soc_info.total_write_bytes));
        println!("├─────────────────────────────────────────────────────────────────────────────┤");
        println!("│ Nodes in this SoC:                                                         │");
        for (i, node) in soc_info.nodes.iter().enumerate() {
            println!("│  {}. {:<70} │", i + 1, node.node_name);
        }
        println!("└─────────────────────────────────────────────────────────────────────────────┘");
    }

    /// Prints summary statistics
    async fn print_summary_stats(&self, data_store: &DataStore) {
        let total_nodes = data_store.get_all_nodes().len();
        let total_socs = data_store.get_all_socs().len();
        let total_boards = data_store.get_all_boards().len();
        
        println!("\n📈 SYSTEM SUMMARY");
        println!("┌─────────────────────────────────────────────────────────────────────────────┐");
        println!("│ Total Nodes: {:<8} │ Total SoCs: {:<8} │ Total Boards: {:<8} │ Status: ✅ │", 
                 total_nodes, total_socs, total_boards);
        
        // Calculate system-wide averages
        let (avg_cpu, avg_mem, total_cores, total_gpus) = self.calculate_system_averages(data_store);
        
        println!("│ System Avg CPU: {:<6.2}% │ Avg Memory: {:<6.2}% │ Total Cores: {:<6} │ GPUs: {:<4} │", 
                 avg_cpu, avg_mem, total_cores, total_gpus);
        println!("└─────────────────────────────────────────────────────────────────────────────┘");
    }

    /// Helper function to format bytes in human-readable format
    fn format_bytes(&self, bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;
        
        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }
        
        if unit_index == 0 {
            format!("{} {}", bytes, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }

    /// Helper function to format memory in human-readable format
    fn format_memory(&self, kb: u64) -> String {
        if kb >= 1024 * 1024 {
            format!("{:.1} GB", kb as f64 / (1024.0 * 1024.0))
        } else if kb >= 1024 {
            format!("{:.1} MB", kb as f64 / 1024.0)
        } else {
            format!("{} KB", kb)
        }
    }

    /// Helper function to format time ago
    fn format_time_ago(&self, time: &std::time::SystemTime) -> String {
        match time.elapsed() {
            Ok(duration) => {
                let secs = duration.as_secs();
                if secs < 60 {
                    format!("{}s ago", secs)
                } else if secs < 3600 {
                    format!("{}m ago", secs / 60)
                } else {
                    format!("{}h ago", secs / 3600)
                }
            }
            Err(_) => "unknown".to_string(),
        }
    }

    /// Helper function to calculate efficiency rating
    fn calculate_efficiency(&self, cpu_usage: f64) -> String {
        if cpu_usage > 90.0 { "HIGH" }
        else if cpu_usage > 70.0 { "GOOD" }
        else if cpu_usage > 30.0 { "NORM" }
        else { "LOW" }.to_string()
    }

    /// Helper function to calculate system-wide averages
    fn calculate_system_averages(&self, data_store: &DataStore) -> (f64, f64, u64, u64) {
        let nodes = data_store.get_all_nodes();
        if nodes.is_empty() {
            return (0.0, 0.0, 0, 0);
        }

        let count = nodes.len() as f64;
        let total_cpu: f64 = nodes.values().map(|n| n.cpu_usage).sum();
        let total_mem: f64 = nodes.values().map(|n| n.mem_usage).sum();
        let total_cores: u64 = nodes.values().map(|n| n.cpu_count).sum();
        let total_gpus: u64 = nodes.values().map(|n| n.gpu_count).sum();

        (total_cpu / count, total_mem / count, total_cores, total_gpus)
    }

    /// Gets a snapshot of all stored data
    pub async fn get_data_snapshot(&self) -> (Vec<NodeInfo>, Vec<SocInfo>, Vec<BoardInfo>) {
        let data_store = self.data_store.lock().await;
        let nodes: Vec<NodeInfo> = data_store.get_all_nodes().values().cloned().collect();
        let socs: Vec<SocInfo> = data_store.get_all_socs().values().cloned().collect();
        let boards: Vec<BoardInfo> = data_store.get_all_boards().values().cloned().collect();
        (nodes, socs, boards)
    }

    /// Print all current data in a comprehensive format
    pub async fn print_all_data(&self) {
        let data_store = self.data_store.lock().await;
        
        println!("\n🌐 COMPLETE SYSTEM OVERVIEW");
        println!("{}", "=".repeat(80));
        
        // Print all nodes
        println!("\n📋 ALL NODES:");
        for (i, (_, node)) in data_store.get_all_nodes().iter().enumerate() {
            println!("{}. {} (IP: {}) - CPU: {:.2}%, Memory: {:.2}%", 
                     i + 1, node.node_name, node.ip, node.cpu_usage, node.mem_usage);
        }
        
        // Print all SoCs
        println!("\n🔧 ALL SOCs:");
        for (i, (_, soc)) in data_store.get_all_socs().iter().enumerate() {
            println!("{}. {} - {} nodes, Avg CPU: {:.2}%, Avg Memory: {:.2}%", 
                     i + 1, soc.soc_id, soc.nodes.len(), soc.total_cpu_usage, soc.total_mem_usage);
        }
        
        // Print all Boards
        println!("\n🖥️  ALL BOARDS:");
        for (i, (_, board)) in data_store.get_all_boards().iter().enumerate() {
            println!("{}. {} - {} nodes, {} SoCs, Avg CPU: {:.2}%, Avg Memory: {:.2}%", 
                     i + 1, board.board_id, board.nodes.len(), board.socs.len(), 
                     board.total_cpu_usage, board.total_mem_usage);
        }
        
        self.print_summary_stats(&data_store).await;
    }

    /// Main loop for processing incoming gRPC ContainerList messages.
    ///
    /// This function continuously receives ContainerList from the gRPC channel
    /// and handles them using the handle_container_list method.
    pub async fn process_container_requests(&self) -> Result<()> {
        loop {
            let container_list_opt = {
                let mut rx_container = self.rx_container.lock().await;
                rx_container.recv().await
            };
            if let Some(container_list) = container_list_opt {
                self.handle_container_list(container_list).await;
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Main loop for processing incoming gRPC NodeInfo messages.
    ///
    /// This function continuously receives NodeInfo from the gRPC channel
    /// and handles them using the handle_node_info method.
    pub async fn process_node_info_requests(&self) -> Result<()> {
        loop {
            let node_info_opt = {
                let mut rx_node = self.rx_node.lock().await;
                rx_node.recv().await
            };
            if let Some(node_info) = node_info_opt {
                self.handle_node_info(node_info).await;
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Runs the MonitoringServerManager event loop.
    ///
    /// Spawns both container and node info processing tasks and waits for them to finish.
    pub async fn run(self) -> Result<()> {
        let arc_self = Arc::new(self);

        // Container processor task
        let container_manager = Arc::clone(&arc_self);
        let container_processor = tokio::spawn(async move {
            if let Err(e) = container_manager.process_container_requests().await {
                eprintln!("Container processor error: {:?}", e);
            }
        });

        // NodeInfo processor task
        let node_manager = Arc::clone(&arc_self);
        let node_processor = tokio::spawn(async move {
            if let Err(e) = node_manager.process_node_info_requests().await {
                eprintln!("Node processor error: {:?}", e);
            }
        });

        let _ = tokio::try_join!(container_processor, node_processor);
        println!("MonitoringServerManager stopped");
        Ok(())
    }
}