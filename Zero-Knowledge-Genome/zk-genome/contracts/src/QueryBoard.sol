// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

/// @title ZK Genome Query Board
/// @notice Pharma posts genomic queries with bounties. Users submit proofs.
contract QueryBoard {
    /// Status of a query.
    enum QueryStatus { Open, Active, Fulfilled, Cancelled }

    /// A genomic query posted by a pharmaceutical researcher.
    struct Query {
        bytes32 queryId;
        address pharma;
        bytes32 proofHash;  // Hash of the encrypted query parameters
        uint256 bounty;     // Total reward pool
        uint256 rewardPerProof;
        uint256 maxSubmissions;
        uint256 submissionCount;
        QueryStatus status;
        uint256 createdAt;
        uint256 deadline;
    }

    /// All queries indexed by ID.
    mapping(bytes32 => Query) public queries;

    /// Pharma's query count.
    mapping(address => uint256) public pharmaQueryCount;

    event QueryPosted(bytes32 indexed queryId, address indexed pharma, uint256 bounty, uint256 deadline);
    event ProofSubmitted(bytes32 indexed queryId, address indexed user);
    event QueryFulfilled(bytes32 indexed queryId, uint256 totalPaid);
    event QueryCancelled(bytes32 indexed queryId);

    /// Pharma posts a new genomic query with a bounty.
    function postQuery(
        bytes32 queryId,
        bytes32 proofHash,
        uint256 bounty,
        uint256 rewardPerProof,
        uint256 maxSubmissions,
        uint256 deadline
    ) external {
        require(bounty > 0, "Bounty must be > 0");
        require(rewardPerProof > 0, "Reward must be > 0");
        require(maxSubmissions > 0, "Must allow submissions");
        require(deadline > block.timestamp, "Deadline must be in future");
        require(queries[queryId].pharma == address(0), "Query ID exists");

        queries[queryId] = Query({
            queryId: queryId,
            pharma: msg.sender,
            proofHash: proofHash,
            bounty: bounty,
            rewardPerProof: rewardPerProof,
            maxSubmissions: maxSubmissions,
            submissionCount: 0,
            status: QueryStatus.Open,
            createdAt: block.timestamp,
            deadline: deadline
        });

        pharmaQueryCount[msg.sender]++;
        emit QueryPosted(queryId, msg.sender, bounty, deadline);
    }

    /// Submit a proof in response to a query.
    function submitProof(bytes32 queryId) external {
        Query storage q = queries[queryId];
        require(q.status == QueryStatus.Open, "Query not open");
        require(block.timestamp < q.deadline, "Past deadline");
        require(q.submissionCount < q.maxSubmissions, "Max submissions reached");

        q.submissionCount++;
        emit ProofSubmitted(queryId, msg.sender);
    }

    /// Pharma marks query as fulfilled after reviewing proofs.
    function fulfillQuery(bytes32 queryId) external {
        Query storage q = queries[queryId];
        require(msg.sender == q.pharma, "Only pharma");
        require(q.status == QueryStatus.Open, "Not open");

        q.status = QueryStatus.Fulfilled;
        emit QueryFulfilled(queryId, q.submissionCount * q.rewardPerProof);
    }

    /// Cancel an open query (before any proofs submitted).
    function cancelQuery(bytes32 queryId) external {
        Query storage q = queries[queryId];
        require(msg.sender == q.pharma, "Only pharma");
        require(q.status == QueryStatus.Open, "Not open");
        require(q.submissionCount == 0, "Has submissions");

        q.status = QueryStatus.Cancelled;
        emit QueryCancelled(queryId);
    }

    /// View query details.
    function getQuery(bytes32 queryId) external view returns (Query memory) {
        return queries[queryId];
    }
}