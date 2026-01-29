# Using Rust Dependencies in Clorus

## Vision: Add Any Rust Crate to Clorus.toml

Just like Cargo, but for Clorus projects!

### Example Clorus.toml

```toml
[package]
name = "my-awesome-app"
version = "0.1.0"
authors = ["Your Name"]

[build]
entry = "src/main.clrs"

[dependencies]
# Regular Rust dependencies
serde_json = "1.0"
reqwest = { version = "0.11", features = ["blocking"] }
regex = "1.9"
chrono = "0.4"
image = "0.24"

[clorus.modules]
# Map Rust crates to Clorus modules
rust.json = "serde_json"
rust.http = "reqwest::blocking"
rust.regex = "regex"
rust.time = "chrono"
rust.image = "image"
```

### Usage in Clorus Code

```clojure
; src/main.clrs
(use rust.json)
(use rust.http)
(use rust.regex)

(defn fetch-and-parse [url]
  (let [response (http/get url)
        body (http/text response)
        data (json/from-str body)]
    data))

(defn extract-emails [text]
  (let [pattern (regex/new "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}")]
    (regex/find-all pattern text)))

(defn main []
  (let [data (fetch-and-parse "https://api.github.com/users/octocat")
        email (get data "email")]
    (println "Email:" email)))

(main)
```

### How It Works

```
1. User writes Clorus.toml
   └─> Lists Rust dependencies

2. clorus build command reads Clorus.toml
   └─> Generates wrapper code automatically
   └─> Compiles Rust code with cargo

3. clorus run command
   └─> Loads all dependency libraries
   └─> Makes functions available to Clorus

4. User's Clorus code
   └─> Uses (use rust.xxx) to access crates
```

## Implementation Design

### 1. Enhanced Clorus.toml Format

```toml
[package]
name = "web-scraper"
version = "0.1.0"

[build]
entry = "src/main.clrs"

[dependencies]
# Rust crates (same format as Cargo.toml)
reqwest = { version = "0.11", features = ["blocking", "json"] }
scraper = "0.17"
serde_json = "1.0"

[clorus.modules]
# Define how Clorus imports them
rust.http = { crate = "reqwest::blocking", functions = ["get", "post"] }
rust.html = { crate = "scraper", functions = ["Html::parse_document", "Selector::parse"] }
rust.json = { crate = "serde_json", functions = ["from_str", "to_string"] }

# Or simple form:
rust.http = "reqwest::blocking"  # Imports all public functions
```

### 2. Auto-Generation Process

When you run `clorus build`:

```rust
// clorus build does this:

1. Parse Clorus.toml
   └─> Read [dependencies] section
   └─> Read [clorus.modules] mappings

2. Generate wrapper crate (clorus-project-deps)
   └─> Create Cargo.toml with all dependencies
   └─> Generate wrappers using clorus-macros
   └─> Example:

       // Generated: clorus-project-deps/src/lib.rs
       use clorus_macros::wrap_module;

       wrap_module! {
           rust.http => reqwest::blocking {
               get(url: String) -> Result<Response>,
               post(url: String) -> Result<Response>,
           }
       }

       wrap_module! {
           rust.json => serde_json {
               from_str(s: String) -> Result<Value>,
               to_string(v: &Value) -> Result<String>,
           }
       }

3. Compile the wrapper crate
   └─> cargo build --release

4. Store metadata for runtime
   └─> Which libraries to load
   └─> Which modules are available
```

### 3. Runtime Loading

When you run `clorus run`:

```rust
// clorus run does this:

1. Load Clorus.toml metadata

2. Load all dependency libraries
   for module in clorus.modules:
       load_library("target/release/libclorus_project_deps.so")

3. Register module mappings
   rust.http -> clorus_http_*
   rust.json -> clorus_json_*

4. Execute Clorus code
   When user does (use rust.http):
   - Check if rust.http is registered ✓
   - Functions already loaded ✓
   - Ready to call!
```

## Real-World Examples

### Example 1: Web Scraper

**Clorus.toml**:
```toml
[package]
name = "web-scraper"

[dependencies]
reqwest = { version = "0.11", features = ["blocking"] }
scraper = "0.17"

[clorus.modules]
rust.http = "reqwest::blocking"
rust.html = "scraper"
```

**src/main.clrs**:
```clojure
(use rust.http)
(use rust.html)

(defn scrape-titles [url]
  (let [response (http/get url)
        html (http/text response)
        document (html/parse html)
        selector (html/selector "h1, h2, h3")]
    (html/select document selector)))

(scrape-titles "https://example.com")
```

### Example 2: Image Processing

**Clorus.toml**:
```toml
[package]
name = "image-processor"

[dependencies]
image = "0.24"

[clorus.modules]
rust.image = "image"
```

**src/main.clrs**:
```clojure
(use rust.image)
(use clorus.core)

(defn resize-image [input output width height]
  (let [img (image/open input)
        resized (image/resize img width height)]
    (image/save resized output)))

(resize-image "large.jpg" "thumb.jpg" 200 200)
```

### Example 3: Database Access

**Clorus.toml**:
```toml
[package]
name = "database-app"

[dependencies]
rusqlite = "0.30"

[clorus.modules]
rust.sqlite = "rusqlite"
```

**src/main.clrs**:
```clojure
(use rust.sqlite)

(defn create-users-table [db]
  (sqlite/execute db "CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT UNIQUE
  )"))

(defn insert-user [db name email]
  (sqlite/execute db
    "INSERT INTO users (name, email) VALUES (?1, ?2)"
    [name email]))

(let [db (sqlite/open "app.db")]
  (create-users-table db)
  (insert-user db "Alice" "alice@example.com"))
```

### Example 4: JSON API Server

**Clorus.toml**:
```toml
[package]
name = "api-server"

[dependencies]
warp = "0.3"
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }

[clorus.modules]
rust.server = "warp"
rust.json = "serde_json"
rust.async = "tokio"
```

**src/main.clrs**:
```clojure
(use rust.server)
(use rust.json)

(defn hello-handler [name]
  (json/to-string {:message (str "Hello, " name "!")}))

(defn main []
  (let [route (server/path "hello")
        handler (server/map route hello-handler)]
    (server/serve handler "127.0.0.1:3000")))

(main)
```

## Command-Line Workflow

```bash
# 1. Create new project
$ clorus new my-web-app
$ cd my-web-app

# 2. Edit Clorus.toml - add dependencies
$ cat >> Clorus.toml << EOF
[dependencies]
reqwest = { version = "0.11", features = ["blocking"] }

[clorus.modules]
rust.http = "reqwest::blocking"
EOF

# 3. Write Clorus code
$ cat > src/main.clrs << EOF
(use rust.http)

(def response (http/get "https://httpbin.org/json"))
(def body (http/text response))
(println body)
EOF

# 4. Build (auto-generates wrappers)
$ clorus build
   Resolving dependencies...
   Generating wrappers for rust.http...
   Compiling clorus-project-deps...
   Finished

# 5. Run
$ clorus run
{"slideshow": {"title": "Sample Slide Show"}}
```

## Implementation Steps

### Week 1: Basic Infrastructure
1. ✅ clorus-macros crate created
2. Implement wrap_module! macro
3. Test with simple functions

### Week 2: Clorus.toml Integration
1. Update Manifest parser to handle [dependencies]
2. Update Manifest parser to handle [clorus.modules]
3. Implement auto-generation in `clorus build`

### Week 3: Runtime Support
1. Auto-load dependencies in `clorus run`
2. Module registration system
3. Error handling for missing dependencies

### Week 4: Polish & Testing
1. Test with popular crates (reqwest, serde_json, regex)
2. Add examples to documentation
3. Create starter templates

## Benefits

✅ **No manual wrapping** - just add to Clorus.toml!
✅ **Access entire Rust ecosystem** - 100,000+ crates
✅ **Familiar workflow** - like Cargo but for Clorus
✅ **Type-safe** - automatic conversions
✅ **Fast compilation** - Rust's speed
✅ **Easy updates** - change version in Clorus.toml

## Comparison with Other Languages

### Clojure (JVM)
```clojure
; In project.clj
:dependencies [[cheshire "5.11.0"]
               [clj-http "3.12.3"]]

; In code
(require '[cheshire.core :as json])
(require '[clj-http.client :as http])
```

### Clorus (Rust)
```clojure
; In Clorus.toml
[dependencies]
serde_json = "1.0"
reqwest = "0.11"

; In code
(use rust.json)
(use rust.http)
```

**Same simplicity, native performance!** 🚀

---

This design makes Clorus **infinitely extensible** - any Rust crate becomes available just by adding it to Clorus.toml!

Ready to implement this? Should I start with the enhanced Clorus.toml parser?
