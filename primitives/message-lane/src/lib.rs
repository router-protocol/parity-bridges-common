// Copyright 2019-2020 Parity Technologies (UK) Ltd.
// This file is part of Parity Bridges Common.

// Parity Bridges Common is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity Bridges Common is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity Bridges Common.  If not, see <http://www.gnu.org/licenses/>.

//! Primitives of message lane module.

#![cfg_attr(not(feature = "std"), no_std)]
// RuntimeApi generated functions
#![allow(clippy::too_many_arguments)]
// Generated by `DecodeLimit::decode_with_depth_limit`
#![allow(clippy::unnecessary_mut_passed)]

use codec::{Decode, Encode};
use frame_support::RuntimeDebug;
use sp_std::{collections::vec_deque::VecDeque, prelude::*};

pub mod source_chain;
pub mod target_chain;

// Weight is reexported to avoid additional frame-support dependencies in message-lane related crates.
pub use frame_support::weights::Weight;

/// Lane identifier.
pub type LaneId = [u8; 4];

/// Message nonce. Valid messages will never have 0 nonce.
pub type MessageNonce = u64;

/// Message id as a tuple.
pub type MessageId = (LaneId, MessageNonce);

/// Opaque message payload. We only decode this payload when it is dispatched.
pub type MessagePayload = Vec<u8>;

/// Message key (unique message identifier) as it is stored in the storage.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct MessageKey {
	/// ID of the message lane.
	pub lane_id: LaneId,
	/// Message nonce.
	pub nonce: MessageNonce,
}

/// Message data as it is stored in the storage.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct MessageData<Fee> {
	/// Message payload.
	pub payload: MessagePayload,
	/// Message delivery and dispatch fee, paid by the submitter.
	pub fee: Fee,
}

/// Message as it is stored in the storage.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct Message<Fee> {
	/// Message key.
	pub key: MessageKey,
	/// Message data.
	pub data: MessageData<Fee>,
}

/// Inbound lane data.
#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
pub struct InboundLaneData<RelayerId> {
	/// Identifiers of relayers and messages that they have delivered (ordered by message nonce).
	/// It is guaranteed to have at most N entries, where N is configured at module level. If
	/// there are N entries in this vec, then:
	/// 1) all incoming messages are rejected if they're missing corresponding `proof-of(outbound-lane.state)`;
	/// 2) all incoming messages are rejected if `proof-of(outbound-lane.state).latest_received_nonce` is
	///    equal to `this.latest_confirmed_nonce`.
	/// Given what is said above, all nonces in this queue are in range:
	/// `(Self::latest_confirmed_nonce(); Self::latest_received_nonce()]`.
	///
	/// When a relayer sends a single message, both of MessageNonces are the same.
	/// When relayer sends messages in a batch, the first arg is the lowest nonce, second arg the highest nonce.
	/// Multiple dispatches from the same relayer one are allowed.
	pub relayers: VecDeque<(MessageNonce, MessageNonce, RelayerId)>,
}

impl<RelayerId> Default for InboundLaneData<RelayerId> {
	fn default() -> Self {
		InboundLaneData {
			relayers: VecDeque::new(),
		}
	}
}

impl<RelayerId> InboundLaneData<RelayerId> {
	/// Nonce of latest message that we have received from bridged chain.
	pub fn latest_received_nonce(&self) -> MessageNonce {
		self.relayers.back()
			.map(|(_, last_nonce, _)| last_nonce)
			.unwrap_or(0)
	}

	/// Nonce of latest message that has been confirmed to the bridged chain.
	pub fn latest_confirmed_nonce(&self) -> MessageNonce {
		self.relayers.front()
			.map(|(first_nonce, ..)| first_nonce.saturating_sub(1))
			.unwrap_or(0)
	}
}

/// Gist of `InboundLaneData::relayers` field used by runtime APIs.
#[derive(Clone, Default, Encode, Decode, RuntimeDebug, PartialEq, Eq)]
pub struct UnrewardedRelayersState {
	/// Number of entries in the `InboundLaneData::relayers` set.
	pub unrewarded_relayer_entries: MessageNonce,
	/// Number of messages in the oldest entry of `InboundLaneData::relayers`. This is the
	/// minimal number of reward proofs required to push out this entry from the set.
	pub messages_in_oldest_entry: MessageNonce,
	/// Total number of messages in the relayers vector.
	pub total_messages: MessageNonce,
}

/// Outbound lane data.
#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
pub struct OutboundLaneData {
	/// Nonce of oldest message that we haven't yet pruned. May point to not-yet-generated message if
	/// all sent messages are already pruned.
	pub oldest_unpruned_nonce: MessageNonce,
	/// Nonce of latest message, received by bridged chain.
	pub latest_received_nonce: MessageNonce,
	/// Nonce of latest message, generated by us.
	pub latest_generated_nonce: MessageNonce,
}

impl Default for OutboundLaneData {
	fn default() -> Self {
		OutboundLaneData {
			// it is 1 because we're pruning everything in [oldest_unpruned_nonce; latest_received_nonce]
			oldest_unpruned_nonce: 1,
			latest_received_nonce: 0,
			latest_generated_nonce: 0,
		}
	}
}

/// Returns total number of messages in the `InboundLaneData::relayers` vector.
pub fn total_unrewarded_messages<RelayerId>(
	relayers: &VecDeque<(MessageNonce, MessageNonce, RelayerId)>,
) -> MessageNonce {
	match (relayers.front(), relayers.back()) {
		(Some((begin, _, _)), Some((_, end, _))) => end.checked_sub(*begin).and_then(|d| d.checked_add(1)).unwrap_or(0),
		_ => 0,
	}
}
