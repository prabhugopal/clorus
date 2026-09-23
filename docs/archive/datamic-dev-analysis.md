> **Archived 2026-02-15 session note**, recovered from an orphaned commit on `origin/main` that never merged into the main development line. May not reflect current code — see `docs/generated/PARITY_STATUS.md` for current status.

⏺ Based on my comprehensive analysis of the Clorus codebase, here's the current state:

  Can You Build a Datomic/MongoDB-like App Today?

  Short Answer: Not yet. You have an excellent foundation but are missing critical I/O and persistence capabilities.

  What You HAVE (Strong Foundation) ✅

  1. Concurrency Model - Production Ready

  - ✅ STM (Software Transactional Memory) - MVCC with optimistic concurrency
  - ✅ Atoms - Lock-free atomic updates
  - ✅ Refs - Coordinated transactional updates
  - ✅ Agents - Async background processing
  - ✅ Channels - CSP-style message passing

  This is Datomic-level concurrency! You have the same model Clojure uses.

  2. Data Structures - Mostly Ready

  - ✅ Persistent Vectors - 32-way trie with structural sharing (like Clojure)
  - ✅ Persistent Lists - Singly-linked immutable lists
  - ⚠️ Maps/Sets - Currently simple hash tables (need HAMT for true persistence)

  3. Query Processing Primitives

  - ✅ Lazy sequences - Memory-efficient iteration
  - ✅ Transducers - Composable transformations
  - ✅ Rich collection API - map, filter, reduce, group-by, etc.

  4. Polymorphism

  - ✅ Protocol system - Type-based dispatch for extensibility

  What's MISSING (Critical Gaps) ❌

  The 5 Critical Blockers:

  1. ❌ File I/O - Can't persist to disk
    - No file open/read/write/close
    - No directory operations
    - No file handles or streams
  2. ❌ Serialization - Can't exchange data
    - No JSON encoding/decoding
    - No binary serialization (EDN, Transit, MessagePack)
    - No data interchange formats
  3. ❌ Networking - Can't build client/server
    - No TCP/UDP sockets
    - No HTTP server/client
    - No network protocols
  4. ❌ Persistent Storage - No durable backend
    - No embedded database integration (RocksDB, SQLite)
    - No transaction log
    - Maps don't persist to disk
  5. ❌ Indexing - No fast lookups
    - No B-trees for range queries
    - No secondary indexes
    - Current maps are simple hash tables

  Gap Analysis: Datomic vs MongoDB

  For Datomic-like Database:
  ┌──────────────────┬──────────────────┬──────────────────────────────┐
  │     Feature      │  Clorus Status   │        What's Missing        │
  ├──────────────────┼──────────────────┼──────────────────────────────┤
  │ Immutable data   │ ✅ Ready         │ None                         │
  ├──────────────────┼──────────────────┼──────────────────────────────┤
  │ MVCC/Time travel │ ✅ STM with refs │ Persist versions to disk     │
  ├──────────────────┼──────────────────┼──────────────────────────────┤
  │ Datalog queries  │ ❌ Missing       │ Full query engine            │
  ├──────────────────┼──────────────────┼──────────────────────────────┤
  │ EAVT indexes     │ ❌ Missing       │ B-tree indexes + persistence │
  ├──────────────────┼──────────────────┼──────────────────────────────┤
  │ Transaction log  │ ❌ Missing       │ Append-only log on disk      │
  ├──────────────────┼──────────────────┼──────────────────────────────┤
  │ Client/server    │ ❌ Missing       │ Networking layer             │
  ├──────────────────┼──────────────────┼──────────────────────────────┤
  │ Peers            │ ✅ Ready         │ STM + atoms work             │
  └──────────────────┴──────────────────┴──────────────────────────────┘
  Gap: ~60% there on in-memory model, 0% on persistence/networking

  For MongoDB-like Database:
  ┌──────────────────┬───────────────┬───────────────────────────┐
  │     Feature      │ Clorus Status │      What's Missing       │
  ├──────────────────┼───────────────┼───────────────────────────┤
  │ Documents (maps) │ ⚠️ Partial    │ Need HAMT for persistence │
  ├──────────────────┼───────────────┼───────────────────────────┤
  │ Collections      │ ✅ Ready      │ None (can use vectors)    │
  ├──────────────────┼───────────────┼───────────────────────────┤
  │ JSON             │ ❌ Missing    │ JSON parser + serializer  │
  ├──────────────────┼───────────────┼───────────────────────────┤
  │ Indexes          │ ❌ Missing    │ B-tree indexes            │
  ├──────────────────┼───────────────┼───────────────────────────┤
  │ Storage engine   │ ❌ Missing    │ WiredTiger-like backend   │
  ├──────────────────┼───────────────┼───────────────────────────┤
  │ Query language   │ ❌ Missing    │ Find/aggregate pipeline   │
  ├──────────────────┼───────────────┼───────────────────────────┤
  │ Replication      │ ❌ Missing    │ Consensus + networking    │
  ├──────────────────┼───────────────┼───────────────────────────┤
  │ Sharding         │ ❌ Missing    │ Distributed coordination  │
  └──────────────────┴───────────────┴───────────────────────────┘
  Gap: ~40% there on data model, 0% on persistence/networking

  Development Roadmap

  Phase 1: Basic Persistence (2-3 weeks)

  ;; Goal: Save and load data

  1. Add File I/O FFI bindings
     - open, read, write, close operations
     - Directory operations

  2. Implement EDN serialization
     - Write Clorus values to text format
     - Read EDN back into values

  ;; After Phase 1, you can:
  (def db (atom {}))
  (swap! db assoc :users [{:name "Alice"}])
  (spit "db.edn" @db)  ; Save to disk
  (def loaded (read-string (slurp "db.edn")))  ; Load back

  Phase 2: Networking (2-3 weeks)

  ;; Goal: Client-server communication

  1. Add TCP socket FFI bindings
     - listen, accept, connect, send, recv

  2. Implement HTTP server
     - Request parsing
     - Response formatting
     - Routing

  ;; After Phase 2, you can:
  (defn handler [req]
    {:status 200
     :body (get @db (:path req))})

  (start-server {:port 8080 :handler handler})

  Phase 3: Storage Backend (3-4 weeks)

  ;; Goal: Production-grade persistence

  1. Integrate RocksDB via FFI
     - Key-value operations
     - Transactions
     - Snapshots

  2. Implement HAMT for persistent maps
     - Replace HashMap with Hash Array Mapped Trie
     - Structural sharing like vectors

  ;; After Phase 3, you can:
  (def db (rocks/open "data.db"))
  (rocks/put db "user:1" {:name "Alice"})
  (rocks/get db "user:1")  ; => {:name "Alice"}

  Phase 4: Query & Indexing (4-6 weeks)

  ;; Goal: Fast lookups and queries

  1. B-tree implementation
     - Sorted key-value store
     - Range scans

  2. Secondary indexes
     - Index on arbitrary fields
     - Composite indexes

  3. Simple query engine
     - Filter predicates
     - Joins
     - Aggregations

  ;; After Phase 4, you can:
  (query db
    {:find [?name]
     :where [[?e :user/age ?age]
             [?e :user/name ?name]
             [(> ?age 18)]]})

  Immediate Next Steps (Priority Order)

  1. File I/O - Week 1 (Highest Priority)

  Add to runtime:
  // crates/clorus-runtime/src/fs.rs
  #[no_mangle]
  pub extern "C" fn clorus_file_read(path: *const c_char) -> *mut Value
  #[no_mangle]
  pub extern "C" fn clorus_file_write(path: *const c_char, content: *mut Value) -> bool

  Clorus API:
  (slurp "data.txt")  ; Read file
  (spit "data.txt" "content")  ; Write file

  2. JSON Serialization - Week 2

  Add to stdlib or as FFI:
  (json/encode {:name "Alice"})  ; => "{\"name\":\"Alice\"}"
  (json/decode "{\"name\":\"Alice\"}")  ; => {:name "Alice"}

  3. TCP Sockets - Week 3-4

  Add networking FFI:
  // crates/clorus-runtime/src/net.rs
  #[no_mangle]
  pub extern "C" fn clorus_tcp_listen(port: i64) -> *mut Value
  #[no_mangle]
  pub extern "C" fn clorus_tcp_accept(socket: *mut Value) -> *mut Value

  Practical Example: What You Can Build TODAY

  ✅ In-Memory Database (Like Redis)

  (ns simple-db)

  ;; Use atoms for concurrent access
  (def db (atom {}))

  ;; Basic operations
  (defn put! [k v]
    (swap! db assoc k v))

  (defn get! [k]
    (get @db k))

  ;; Transactions with STM
  (def accounts (ref {}))

  (defn transfer [from to amount]
    (dosync
      (alter accounts update from - amount)
      (alter accounts update to + amount)))

  ;; Async indexing with agents
  (def index-agent (agent {}))

  (defn index-doc [doc]
    (send index-agent assoc (:id doc) doc))

  This works TODAY - but only in-memory!

  Recommendation

  For a Datomic-like database:

  1. Start with Phase 1 (File I/O) - Critical for any database
  2. Then Phase 2 (Networking) - Enables client/server
  3. Then Phase 3 (Storage Backend) - Production durability
  4. Finally Phase 4 (Indexing & Query) - Performance

  Total Estimate: 3-4 months of focused development

  Alternative: You could build a pure in-memory database TODAY and use Clorus's excellent concurrency model. Think Redis-like, not Datomic-like.

  Want me to help you start with Phase 1 (File I/O)? That's the foundation for everything else.
