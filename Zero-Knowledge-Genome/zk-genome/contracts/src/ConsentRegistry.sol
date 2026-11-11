// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

/// @title ZK Genome Consent Registry
/// @notice On-chain consent verification for genomic data usage.
///         Users grant/revoke consent via DID-based credentials.
contract ConsentRegistry {
    /// A consent credential granted by a user.
    struct Consent {
        bytes32 userId;
        bytes32 queryId;
        bytes32 didHash;
        uint256 grantedAt;
        uint256 expiresAt;
        bool revoked;
        bytes32 scope; // What data is permitted
    }

    /// userId => queryId => Consent
    mapping(bytes32 => mapping(bytes32 => Consent)) public consents;

    /// userId => consent count
    mapping(bytes32 => uint256) public userConsentCount;

    event ConsentGranted(bytes32 indexed userId, bytes32 indexed queryId, bytes32 didHash, uint256 expiresAt);
    event ConsentRevoked(bytes32 indexed userId, bytes32 indexed queryId);

    /// Grant consent for a specific query.
    function grantConsent(
        bytes32 queryId,
        bytes32 didHash,
        uint256 durationSeconds,
        bytes32 scope
    ) external {
        bytes32 userId = keccak256(abi.encode(msg.sender, didHash));
        require(consents[userId][queryId].grantedAt == 0, "Consent already exists");

        consents[userId][queryId] = Consent({
            userId: userId,
            queryId: queryId,
            didHash: didHash,
            grantedAt: block.timestamp,
            expiresAt: block.timestamp + durationSeconds,
            revoked: false,
            scope: scope
        });

        userConsentCount[userId]++;
        emit ConsentGranted(userId, queryId, didHash, block.timestamp + durationSeconds);
    }

    /// Revoke consent for a specific query.
    function revokeConsent(bytes32 queryId, bytes32 didHash) external {
        bytes32 userId = keccak256(abi.encode(msg.sender, didHash));
        require(consents[userId][queryId].grantedAt > 0, "Consent not found");
        require(!consents[userId][queryId].revoked, "Already revoked");

        consents[userId][queryId].revoked = true;
        emit ConsentRevoked(userId, queryId);
    }

    /// Check if consent is valid for a userId + queryId.
    function isConsentValid(bytes32 userId, bytes32 queryId) external view returns (bool) {
        Consent storage c = consents[userId][queryId];
        return c.grantedAt > 0 && !c.revoked && block.timestamp < c.expiresAt;
    }

    /// Get consent details.
    function getConsent(bytes32 userId, bytes32 queryId) external view returns (Consent memory) {
        return consents[userId][queryId];
    }
}