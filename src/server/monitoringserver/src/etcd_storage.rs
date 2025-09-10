/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Store and retrieve monitoring data in etcd

use crate::data_structures::{BoardInfo, SocInfo};
use common::monitoringserver::NodeInfo;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct SerializableNodeInfo {
    pub node_name: String,
    pub ip: String,
    pub cpu_usage: f64,
    pub cpu_count: u64,
    pub gpu_count: u64,
    pub used_memory: u64,
    pub total_memory: u64,
    pub mem_usage: f64,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub os: String,
    pub arch: String,
}

impl From<&NodeInfo> for SerializableNodeInfo {
    fn from(node_info: &NodeInfo) -> Self {
        Self {
            node_name: node_info.node_name.clone(),
            ip: node_info.ip.clone(),
            cpu_usage: node_info.cpu_usage,
            cpu_count: node_info.cpu_count,
            gpu_count: node_info.gpu_count,
            used_memory: node_info.used_memory,
            total_memory: node_info.total_memory,
            mem_usage: node_info.mem_usage,
            rx_bytes: node_info.rx_bytes,
            tx_bytes: node_info.tx_bytes,
            read_bytes: node_info.read_bytes,
            write_bytes: node_info.write_bytes,
            os: node_info.os.clone(),
            arch: node_info.arch.clone(),
        }
    }
}

impl From<SerializableNodeInfo> for NodeInfo {
    fn from(serializable: SerializableNodeInfo) -> Self {
        Self {
            node_name: serializable.node_name,
            ip: serializable.ip,
            cpu_usage: serializable.cpu_usage,
            cpu_count: serializable.cpu_count,
            gpu_count: serializable.gpu_count,
            used_memory: serializable.used_memory,
            total_memory: serializable.total_memory,
            mem_usage: serializable.mem_usage,
            rx_bytes: serializable.rx_bytes,
            tx_bytes: serializable.tx_bytes,
            read_bytes: serializable.read_bytes,
            write_bytes: serializable.write_bytes,
            os: serializable.os,
            arch: serializable.arch,
        }
    }
}

/// Store NodeInfo in etcd
pub async fn store_node_info(node_info: &NodeInfo) -> common::Result<()> {
    let key = format!("monitoring/nodes/{}", node_info.node_name);
    let serializable = SerializableNodeInfo::from(node_info);
    let json_data = serde_json::to_string(&serializable)
        .map_err(|e| format!("Failed to serialize NodeInfo: {}", e))?;

    common::etcd::put(&key, &json_data).await?;
    println!("[ETCD] Stored NodeInfo for node: {}", node_info.node_name);
    Ok(())
}

/// Store SocInfo in etcd
pub async fn store_soc_info(soc_info: &SocInfo) -> common::Result<()> {
    let key = format!("monitoring/socs/{}", soc_info.soc_id);
    let json_data = serde_json::to_string(soc_info)
        .map_err(|e| format!("Failed to serialize SocInfo: {}", e))?;

    common::etcd::put(&key, &json_data).await?;
    println!("[ETCD] Stored SocInfo for SoC: {}", soc_info.soc_id);
    Ok(())
}

/// Store BoardInfo in etcd
pub async fn store_board_info(board_info: &BoardInfo) -> common::Result<()> {
    let key = format!("monitoring/boards/{}", board_info.board_id);
    let json_data = serde_json::to_string(board_info)
        .map_err(|e| format!("Failed to serialize BoardInfo: {}", e))?;

    common::etcd::put(&key, &json_data).await?;
    println!("[ETCD] Stored BoardInfo for board: {}", board_info.board_id);
    Ok(())
}

/// Retrieve NodeInfo from etcd
pub async fn get_node_info(node_name: &str) -> common::Result<NodeInfo> {
    let key = format!("monitoring/nodes/{}", node_name);
    let json_data = common::etcd::get(&key).await?;

    let serializable: SerializableNodeInfo = serde_json::from_str(&json_data)
        .map_err(|e| format!("Failed to deserialize NodeInfo: {}", e))?;

    Ok(NodeInfo::from(serializable))
}

/// Retrieve SocInfo from etcd
pub async fn get_soc_info(soc_id: &str) -> common::Result<SocInfo> {
    let key = format!("monitoring/socs/{}", soc_id);
    let json_data = common::etcd::get(&key).await?;

    let soc_info: SocInfo = serde_json::from_str(&json_data)
        .map_err(|e| format!("Failed to deserialize SocInfo: {}", e))?;

    Ok(soc_info)
}

/// Retrieve BoardInfo from etcd
pub async fn get_board_info(board_id: &str) -> common::Result<BoardInfo> {
    let key = format!("monitoring/boards/{}", board_id);
    let json_data = common::etcd::get(&key).await?;

    let board_info: BoardInfo = serde_json::from_str(&json_data)
        .map_err(|e| format!("Failed to deserialize BoardInfo: {}", e))?;

    Ok(board_info)
}

/// Get all nodes from etcd
pub async fn get_all_nodes() -> common::Result<Vec<NodeInfo>> {
    let kv_pairs = common::etcd::get_all_with_prefix("monitoring/nodes/").await?;

    let mut nodes = Vec::with_capacity(kv_pairs.len()); // Pre-allocate
    for kv in kv_pairs {
        match serde_json::from_str::<SerializableNodeInfo>(&kv.value) {
            // Use SerializableNodeInfo
            Ok(serializable) => nodes.push(NodeInfo::from(serializable)),
            Err(e) => eprintln!("[ETCD] Failed to deserialize node {}: {}", kv.key, e),
        }
    }

    Ok(nodes)
}

/// Get all SoCs from etcd
pub async fn get_all_socs() -> common::Result<Vec<SocInfo>> {
    let kv_pairs = common::etcd::get_all_with_prefix("monitoring/socs/").await?;

    let mut socs = Vec::new();
    for kv in kv_pairs {
        match serde_json::from_str::<SocInfo>(&kv.value) {
            Ok(soc_info) => socs.push(soc_info),
            Err(e) => eprintln!("[ETCD] Failed to deserialize SoC {}: {}", kv.key, e),
        }
    }

    Ok(socs)
}

/// Get all boards from etcd
pub async fn get_all_boards() -> common::Result<Vec<BoardInfo>> {
    let kv_pairs = common::etcd::get_all_with_prefix("monitoring/boards/").await?;

    let mut boards = Vec::new();
    for kv in kv_pairs {
        match serde_json::from_str::<BoardInfo>(&kv.value) {
            Ok(board_info) => boards.push(board_info),
            Err(e) => eprintln!("[ETCD] Failed to deserialize board {}: {}", kv.key, e),
        }
    }

    Ok(boards)
}

/// Delete NodeInfo from etcd
pub async fn delete_node_info(node_name: &str) -> common::Result<()> {
    let key = format!("monitoring/nodes/{}", node_name);
    common::etcd::delete(&key).await?;
    println!("[ETCD] Deleted NodeInfo for node: {}", node_name);
    Ok(())
}

/// Delete SocInfo from etcd
pub async fn delete_soc_info(soc_id: &str) -> common::Result<()> {
    let key = format!("monitoring/socs/{}", soc_id);
    common::etcd::delete(&key).await?;
    println!("[ETCD] Deleted SocInfo for SoC: {}", soc_id);
    Ok(())
}

/// Delete BoardInfo from etcd
pub async fn delete_board_info(board_id: &str) -> common::Result<()> {
    let key = format!("monitoring/boards/{}", board_id);
    common::etcd::delete(&key).await?;
    println!("[ETCD] Deleted BoardInfo for board: {}", board_id);
    Ok(())
}
