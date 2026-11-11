// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

/// @title ZK Genome Proof Registry
/// @notice Stores proof hashes on-chain to prevent replay attacks.
///         Each proof is linked to its query and submitting user.
contract ProofRegistry {
    /// A verified zero-knowledge proof submission.
    struct ProofRecord {
        bytes32 proofHash;
        bytes32 queryId;
        address user;
        uint256 timestamp;
        bool verified;
    }

    /// proofHash => ProofRecord
    mapping(bytes32 => ProofRecord) public proofs;

    /// queryId => proofHash[] (for enumeration)
    mapping(bytes32 => bytes32[]) public queryProofs;

    /// Total proofs registered.
    uint256 public totalProofs;

    event ProofRegistered(bytes32 indexed proofHash, bytes32 indexed queryId, address indexed user);
    event ProofVerified(bytes32 indexed proofHash, bool valid);

    /// Register a new proof hash for a query.
    function registerProof(bytes32 proofHash, bytes32 queryId) external {
        require(proofs[proofHash].user == address(0), "Proof already registered");
        require(proofHash != bytes32(0), "Invalid proof hash");

        proofs[proofHash] = ProofRecord({
            proofHash: proofHash,
            queryId: queryId,
            user: msg.sender,
            timestamp: block.timestamp,
            verified: false
        });

        queryProofs[queryId].push(proofHash);
        totalProofs++;

        emit ProofRegistered(proofHash, queryId, msg.sender);
    }

    /// Mark a proof as verified by a verification node.
    function verifyProof(bytes32 proofHash, bool valid) external {
        require(proofs[proofHash].user != address(0), "Proof not found");
        require(!proofs[proofHash].verified, "Already verified");

        proofs[proofHash].verified = true;
        emit ProofVerified(proofHash, valid);
    }

    /// Check if a proof exists and is verified.
    function isProofVerified(bytes32 proofHash) external view returns (bool) {
        return proofs[proofHash].verified;
    }

    /// Get all proof hashes for a query.
    function getQueryProofs(bytes32 queryId) external view returns (bytes32[] memory) {
        return queryProofs[queryId];
    }

    /// Get the number of verified proofs for a query.
    function getVerifiedCount(bytes32 queryId) external view returns (uint256) {
        bytes32[] storage hashes = queryProofs[queryId];
        uint256 count;
        for (uint256 i = 0; i < hashes.length; i++) {
            if (proofs[hashes[i]].verified) {
                count++;
            }
        }
        return count;
    }
}