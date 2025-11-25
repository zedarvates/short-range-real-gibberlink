use crate::crypto::CryptoEngine;
use crate::protocol::{ProtocolEngine, ProtocolError};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, serde::Serialize, serde::Deserialize)]
pub enum MilitaryRank {
    Private = 1,     // Level 1
    Sergeant = 2,    // Level 2
    Lieutenant = 3,  // Level 3
    Captain = 4,     // Level 4
    Commander = 5,   // Level 5
    Colonel = 6,     // Level 6
    General = 7,     // Level 7
}

impl MilitaryRank {
    pub fn from_level(level: u8) -> Option<Self> {
        match level {
            1 => Some(MilitaryRank::Private),
            2 => Some(MilitaryRank::Sergeant),
            3 => Some(MilitaryRank::Lieutenant),
            4 => Some(MilitaryRank::Captain),
            5 => Some(MilitaryRank::Commander),
            6 => Some(MilitaryRank::Colonel),
            7 => Some(MilitaryRank::General),
            _ => None,
        }
    }

    pub fn level(&self) -> u8 {
        self.clone() as u8
    }

    pub fn can_command(&self, target: &MilitaryRank) -> bool {
        self >= target
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CommandType {
    DirectOrder,
    CoordinationOrder,
    StatusUpdate,
    Acknowledgment,
    Emergency,
    SyncRequest,
    FormationCommand,
    ResourceAllocation,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HierarchicalMessage {
    pub rank: MilitaryRank,
    pub target_rank: Option<MilitaryRank>,
    pub sequence_id: u32,
    pub timestamp: u64,
    pub command_type: CommandType,
    pub payload: Vec<u8>,
    pub ack_required: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum HierarchicalState {
    Idle,
    Discovering,
    EstablishingRank,
    ActiveCommand,
    EmergencyMode,
}

#[derive(Debug, Clone)]
pub struct HierarchyPresence {
    pub rank: MilitaryRank,
    pub device_id: [u8; 16],
    pub last_seen: Instant,
    pub active: bool,
}

pub struct HierarchicalProtocolEngine {
    state: Arc<Mutex<HierarchicalState>>,
    my_rank: MilitaryRank,
    presence_list: Arc<Mutex<Vec<HierarchyPresence>>>,
    protocol_engine: Arc<Mutex<ProtocolEngine>>,
    hierarchical_sequence: u32,
    discovery_interval: Duration,
    last_discovery: Instant,
    superior_timeout: Duration,
}

impl HierarchicalProtocolEngine {
    pub fn new(protocol_engine: Arc<Mutex<ProtocolEngine>>, rank: MilitaryRank) -> Self {
        Self {
            state: Arc::new(Mutex::new(HierarchicalState::Idle)),
            my_rank: rank,
            presence_list: Arc::new(Mutex::new(Vec::new())),
            protocol_engine,
            hierarchical_sequence: 0,
            discovery_interval: Duration::from_secs(5),
            last_discovery: Instant::now(),
            superior_timeout: Duration::from_millis(800),
        }
    }

    pub async fn enable_hierarchy(&self) -> Result<(), ProtocolError> {
        let mut state = self.state.lock().await;
        *state = HierarchicalState::Discovering;
        Ok(())
    }

    pub async fn broadcast_rank_presence(&mut self) -> Result<(), ProtocolError> {
        if Instant::now().duration_since(self.last_discovery) < self.discovery_interval {
            return Ok(());
        }

        let message = HierarchicalMessage {
            rank: self.my_rank.clone(),
            target_rank: None, // Broadcast
            sequence_id: self.hierarchical_sequence,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            command_type: CommandType::StatusUpdate,
            payload: format!("RANK_{}", self.my_rank.level()).into_bytes(),
            ack_required: false,
        };

        self.hierarchical_sequence += 1;

        // Send via ultrasound for rank broadcasting
        let mut protocol = self.protocol_engine.lock().await;
        let audio = protocol.get_audio_engine_mut();
        audio.send_data(&serde_json::to_vec(&message).map_err(|_| {
            ProtocolError::CryptoError("Failed to serialize rank message".into())
        })?).await
        .map_err(|e| ProtocolError::AudioError(e.to_string()))?;

        self.last_discovery = Instant::now();
        Ok(())
    }

    pub async fn receive_hierarchical_message(&mut self, data: &[u8]) -> Result<(), ProtocolError> {
        let message: HierarchicalMessage = serde_json::from_slice(data).map_err(|_| {
            ProtocolError::CryptoError("Invalid hierarchical message".into())
        })?;

        // Validate rank hierarchy
        if message.rank > self.my_rank {
            // Higher rank detected - update presence and potentially defer command
            self.update_presence_list(message.rank, [0u8; 16], true).await?;
            self.check_superior_failover().await?;
        } else if message.rank <= self.my_rank && message.target_rank.clone().map_or(true, |target| target >= self.my_rank) {
            // Accept commands from equal or higher ranks, or broadcasts
            self.process_command(message).await?;
        }

        Ok(())
    }

    pub async fn send_command(&mut self, target_rank: MilitaryRank, command_type: CommandType, payload: Vec<u8>, require_ack: bool) -> Result<u32, ProtocolError> {
        // Only allow commanding lower or equal ranks
        if !self.my_rank.can_command(&target_rank) {
            return Err(ProtocolError::InvalidState);
        }

        let sequence = self.hierarchical_sequence;
        let message = HierarchicalMessage {
            rank: self.my_rank.clone(),
            target_rank: Some(target_rank),
            sequence_id: sequence,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            command_type,
            payload,
            ack_required: require_ack,
        };

        self.hierarchical_sequence += 1;

        // Send via appropriate channel based on rank communication preferences
        let mut protocol = self.protocol_engine.lock().await;
        let message_data = serde_json::to_vec(&message).map_err(|_| {
            ProtocolError::CryptoError("Failed to serialize command".into())
        })?;

        match self.my_rank {
            MilitaryRank::General | MilitaryRank::Colonel => {
                // Use laser for high-level commands
                if let Some(laser) = protocol.get_laser_engine_mut() {
                    laser.transmit_data(&message_data).await
                        .map_err(|e| ProtocolError::LaserError(e))?;
                }
            }
            MilitaryRank::Commander | MilitaryRank::Captain => {
                // Use ultrasound + LED for mid-level
                let audio = protocol.get_audio_engine_mut();
                audio.send_data(&message_data).await
                    .map_err(|e| ProtocolError::AudioError(e.to_string()))?;
            }
            _ => {
                // Use basic ultrasound for lower ranks
                let audio = protocol.get_audio_engine_mut();
                audio.send_data(&message_data).await
                    .map_err(|e| ProtocolError::AudioError(e.to_string()))?;
            }
        }

        Ok(sequence)
    }

    pub async fn coordinate_multi_device(&mut self, target_ranks: Vec<MilitaryRank>, command: &str) -> Result<(), ProtocolError> {
        // Send simultaneous commands to multiple devices
        let payload = command.as_bytes().to_vec();
        let mut handles = vec![];

        for rank in target_ranks {
            let self_clone = Arc::new(Mutex::new(self.clone()));
            let payload_clone = payload.clone();

            let handle = tokio::spawn(async move {
                let mut engine = self_clone.lock().await;
                engine.send_command(rank, CommandType::CoordinationOrder, payload_clone, true).await
            });
            handles.push(handle);
        }

        // Wait for all commands to be sent
        for handle in handles {
            handle.await.map_err(|_| ProtocolError::CryptoError("Coordination failed".into()))??;
        }

        Ok(())
    }

    async fn update_presence_list(&mut self, rank: MilitaryRank, device_id: [u8; 16], active: bool) -> Result<(), ProtocolError> {
        let mut list = self.presence_list.lock().await;
        let now = Instant::now();

        // Remove expired entries
        list.retain(|p| now.duration_since(p.last_seen) < Duration::from_secs(30));

        // Update or add presence
        if let Some(existing) = list.iter_mut().find(|p| p.rank == rank && p.device_id == device_id) {
            existing.last_seen = now;
            existing.active = active;
        } else {
            list.push(HierarchyPresence {
                rank,
                device_id,
                last_seen: now,
                active,
            });
        }

        Ok(())
    }

    async fn check_superior_failover(&mut self) -> Result<(), ProtocolError> {
        let list = self.presence_list.lock().await;
        let now = Instant::now();

        // Check if any superior disappeared
        let has_superior = list.iter().any(|p|
            p.rank > self.my_rank &&
            p.active &&
            now.duration_since(p.last_seen) < self.superior_timeout
        );

        if !has_superior {
            // No superior present - take command
            let mut state = self.state.lock().await;
            *state = HierarchicalState::ActiveCommand;

            // Optionally broadcast leadership assumption
            drop(state);
            drop(list);
            self.broadcast_rank_presence().await?;
        }

        Ok(())
    }

    async fn process_command(&mut self, message: HierarchicalMessage) -> Result<(), ProtocolError> {
        match message.command_type {
            CommandType::DirectOrder | CommandType::CoordinationOrder => {
                // Process order and potentially send ACK
                if message.ack_required {
                    self.send_ack(message.sequence_id).await?;
                }

                // Execute command based on payload
                self.execute_command_payload(&message.payload).await?;
            }
            CommandType::Emergency => {
                // Switch to emergency mode
                let mut state = self.state.lock().await;
                *state = HierarchicalState::EmergencyMode;
            }
            _ => {
                // Handle other command types
            }
        }

        Ok(())
    }

    async fn send_ack(&mut self, sequence_id: u32) -> Result<(), ProtocolError> {
        let message = HierarchicalMessage {
            rank: self.my_rank.clone(),
            target_rank: None, // ACK to whoever sent the command
            sequence_id: self.hierarchical_sequence,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            command_type: CommandType::Acknowledgment,
            payload: format!("ACK_{}", sequence_id).into_bytes(),
            ack_required: false,
        };

        self.hierarchical_sequence += 1;

        // Send ACK via ultrasound
        let mut protocol = self.protocol_engine.lock().await;
        let audio = protocol.get_audio_engine_mut();
        audio.send_data(&serde_json::to_vec(&message).map_err(|_| {
            ProtocolError::CryptoError("Failed to serialize ACK".into())
        })?).await
        .map_err(|e| ProtocolError::AudioError(e.to_string()))?;

        Ok(())
    }

    async fn execute_command_payload(&self, payload: &[u8]) -> Result<(), ProtocolError> {
        let command = String::from_utf8_lossy(payload);

        if command.starts_with("PUSH CART") {
            // Example: handle cart pushing coordination
            // In real implementation, this would interface with robot control systems
            println!("Hierarchical command executed: {}", command);
        } else if command.starts_with("SYNC") {
            // Handle synchronization commands
            println!("Synchronization command: {}", command);
        } else if command == "FOLLOW GREEN FLASHES" {
            // Emergency mode: follow highest rank robot flashes
            println!("Entering emergency autonomous mode");
        }

        Ok(())
    }

    pub async fn get_highest_rank_present(&self) -> Option<MilitaryRank> {
        let list = self.presence_list.lock().await;
        list.iter()
            .filter(|p| p.active)
            .map(|p| p.rank.clone())
            .max()
    }

    pub async fn is_superior_present(&self) -> bool {
        let list = self.presence_list.lock().await;
        let now = Instant::now();

        list.iter().any(|p|
            p.rank > self.my_rank &&
            p.active &&
            now.duration_since(p.last_seen) < self.superior_timeout
        )
    }

    pub async fn get_current_state(&self) -> HierarchicalState {
        self.state.lock().await.clone()
    }

    pub fn get_rank(&self) -> &MilitaryRank {
        &self.my_rank
    }
}

impl Clone for HierarchicalProtocolEngine {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            my_rank: self.my_rank.clone(),
            presence_list: Arc::clone(&self.presence_list),
            protocol_engine: Arc::clone(&self.protocol_engine),
            hierarchical_sequence: self.hierarchical_sequence,
            discovery_interval: self.discovery_interval,
            last_discovery: self.last_discovery,
            superior_timeout: self.superior_timeout,
        }
    }
}
