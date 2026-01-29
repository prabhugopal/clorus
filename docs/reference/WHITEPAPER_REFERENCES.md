# Essential Whitepapers & References for Clorus

## Core Philosophy: Keep It Simple

Before diving into papers, remember Rich Hickey's (Clojure creator) philosophy:
- **Simple vs Easy**: Simple means "not intertwined", not "familiar"
- **Data-oriented**: Programs manipulate immutable data
- **Persistent data structures**: Efficient immutability through structural sharing

---

## 1. Persistent Data Structures (CRITICAL)

### 📄 "Purely Functional Data Structures" - Chris Okasaki (1996)
**The Bible of Persistent Data Structures**

- **Link**: Available as book and PhD thesis
- **Why Essential**: Foundation for all functional data structures
- **Key Concepts**:
  - Lazy evaluation for amortization
  - Path copying
  - Incremental algorithms
- **For Clorus**: Theoretical foundation for vectors, lists, trees

```
@phdthesis{okasaki1996purely,
  title={Purely functional data structures},
  author={Okasaki, Chris},
  year={1996},
  school={Carnegie Mellon University}
}
```

**Book**: "Purely Functional Data Structures" (Cambridge University Press, 1998)
- Readable, practical
- Code examples in ML and Haskell
- Chapters on queues, heaps, search trees

### 📄 "Ideal Hash Trees" - Phil Bagwell (2001)
**The Paper Behind Clojure's PersistentHashMap**

- **PDF**: https://infoscience.epfl.ch/record/64398/files/idealhashtrees.pdf
- **Why Essential**: This IS what Clojure uses for hashmaps
- **Key Concepts**:
  - Hash Array Mapped Trie (HAMT)
  - Bitmap indexing
  - 32-way branching
  - Structural sharing on updates
- **For Clorus**: Direct blueprint for HashMap implementation

```
@inproceedings{bagwell2001ideal,
  title={Ideal hash trees},
  author={Bagwell, Phil},
  booktitle={Es Grands Champs},
  volume={1195},
  year={2001}
}
```

**Key Quote**: "A persistent data structure preserves all previous versions of itself when modified."

### 📄 "RRB-Trees: Efficient Immutable Vectors" - Stucki et al. (2015)
**Advanced Persistent Vectors**

- **PDF**: https://infoscience.epfl.ch/record/213452/files/rrbvector.pdf
- **Why Important**: Improves on Clojure's vectors
- **Key Concepts**:
  - Relaxed Radix Balanced trees
  - O(log n) concatenation (vs Clojure's O(n))
  - Better split/slice operations
- **For Clorus**: Future optimization, not needed initially

```
@article{stucki2015rrb,
  title={RRB-Trees: Efficient Immutable Vectors},
  author={Stucki, Nicolas and Rompf, Tiark and Ureche, Vlad and Bagwell, Phil},
  journal={ACM SIGPLAN Notices},
  volume={50},
  number={9},
  pages={549--561},
  year={2015}
}
```

---

## 2. Memory Management

### 📄 "Unified Theory of Garbage Collection" - Bacon et al. (2004)
**Comprehensive GC Overview**

- **PDF**: https://www.cs.cornell.edu/courses/cs6120/2019fa/blog/unified-theory-gc/
- **Why Essential**: Unifies tracing and reference counting
- **Key Insight**: RC and GC are duals of each other
- **For Clorus**: Understand trade-offs, design for future

```
@inproceedings{bacon2004unified,
  title={A unified theory of garbage collection},
  author={Bacon, David F and Cheng, Perry and Rajan, VT},
  booktitle={ACM SIGPLAN Notices},
  volume={39},
  number={10},
  pages={50--68},
  year={2004}
}
```

**Key Takeaway**:
- Tracing = find live objects, free dead ones
- RC = find dead objects directly
- Both can be optimized similarly

### 📄 "Reference Counting with Frame Limited Reuse" - Ullrich & Moura (2019)
**Modern RC from Lean 4**

- **PDF**: https://arxiv.org/pdf/1908.05647.pdf
- **Why Important**: Shows RC can be competitive with GC
- **Key Concepts**:
  - Reuse analysis at compile time
  - In-place updates when safe
  - No atomic operations for single-threaded
- **For Clorus**: Future optimization for RC

```
@article{ullrich2019counting,
  title={Counting Immutable Beans: Reference Counting Optimized for Purely Functional Programming},
  author={Ullrich, Sebastian and de Moura, Leonardo},
  journal={arXiv preprint arXiv:1908.05647},
  year={2019}
}
```

**Relevance**: Shows RC can work well for functional languages!

### 📄 "Perceus: Garbage Free Reference Counting with Reuse" - Reinking et al. (2021)
**State-of-the-Art RC**

- **PDF**: https://www.microsoft.com/en-us/research/uploads/prod/2021/06/perceus-pldi21.pdf
- **Why Important**: Latest advances in RC
- **Key Concepts**:
  - Precise automatic reuse
  - Drop specialization
  - Competitive with tracing GC
- **For Clorus**: Advanced future work

---

## 3. Clojure Internals

### 📄 "The Clojure Programming Language" - Rich Hickey (2008)
**Original Vision**

- **Video**: "Clojure for Java Programmers" (2008) - YouTube
- **Slides**: Available online
- **Why Essential**: Understand design philosophy
- **Key Points**:
  - Immutability by default
  - Persistent data structures
  - Hosted language (JVM)
  - Simplicity over complexity

### 📄 Clojure Source Code
**The Best Documentation**

- **Repo**: https://github.com/clojure/clojure
- **Files to Study**:
  - `src/jvm/clojure/lang/PersistentVector.java` - Vector implementation
  - `src/jvm/clojure/lang/PersistentHashMap.java` - HashMap (HAMT)
  - `src/jvm/clojure/lang/PersistentList.java` - Simple linked list
- **Why Essential**: See actual production implementation
- **For Clorus**: Direct translation guide

**Key Code Patterns**:
```java
// From PersistentVector.java
static final int BRANCHING_BIT = 5;  // 2^5 = 32
static final int BRANCHING_FACTOR = 1 << BRANCHING_BIT;  // 32

// Bitmap trick from PersistentHashMap
static int mask(int hash, int shift) {
    return (hash >>> shift) & 0x01f;  // Get 5 bits
}
```

### 📄 "Clojure Applied" - Ben Vandgrift & Alex Miller (2015)
**Practical Patterns**

- **Book**: O'Reilly
- **Why Useful**: Real-world usage patterns
- **For Clorus**: Understand how features are actually used

---

## 4. Language Design

### 📄 "Simple Made Easy" - Rich Hickey (2011)
**Philosophy**

- **Video**: https://www.infoq.com/presentations/Simple-Made-Easy/
- **Transcript**: Available online
- **Why Critical**: Core philosophy behind Clojure
- **Key Distinctions**:
  - Simple ≠ Easy
  - Simple = one purpose, one concept, one role
  - Complect = intertwine, braid together (BAD)

**Quote**: "Simplicity is a prerequisite for reliability."

### 📄 "The Value of Values" - Rich Hickey (2012)
**Immutability Rationale**

- **Video**: https://www.youtube.com/watch?v=-6BsiVyC1kM
- **Why Important**: Why immutable data structures matter
- **Key Concepts**:
  - Values don't change
  - Time as succession of values
  - Perception vs memory

### 📄 "Are We There Yet?" - Rich Hickey (2009)
**Identity and State**

- **Video**: Available on InfoQ
- **Why Relevant**: How to think about time and state
- **For Clorus**: Atom/Ref design inspiration

---

## 5. LLVM & Code Generation

### 📄 "LLVM: A Compilation Framework for Lifelong Program Analysis"
**LLVM Foundation**

- **PDF**: https://llvm.org/pubs/2004-01-30-CGO-LLVM.pdf
- **Authors**: Lattner & Adve (2004)
- **Why Essential**: Understand LLVM IR
- **For Clorus**: Backend target

### 📄 "Kaleidoscope Tutorial" - LLVM Docs
**Practical LLVM**

- **Link**: https://llvm.org/docs/tutorial/
- **Why Essential**: Learn by doing
- **Covers**: Lexer, parser, AST, codegen, JIT
- **For Clorus**: Similar to what we're building!

---

## 6. Type Systems (Future)

### 📄 "Practical Type Inference for Arbitrary-Rank Types" - Peyton Jones et al. (2007)
**Advanced Type Inference**

- **PDF**: Available from Microsoft Research
- **Why Relevant**: If we add static typing
- **For Clorus**: Future optional typing system

### 📄 "Typed Clojure" - Ambrose Bonnaire-Sergeant (2012-2016)
**Optional Typing for Clojure**

- **Thesis**: https://repository.library.northeastern.edu/files/neu:cj82qf18d
- **Why Useful**: See how typing can fit Clojure
- **For Clorus**: Design gradual typing

---

## 7. Debugging & Visualization

### 📄 "Visualizing Memory Management in Python" - Philip Guo
**Pedagogical Visualization**

- **Tool**: Python Tutor (pythontutor.com)
- **Why Relevant**: Inspiration for our debug mode
- **Concept**: Step-by-step execution visualization

### 📄 "GCspy: An Adaptable Heap Visualisation Framework" - Printezis & Jones (2002)
**GC Visualization**

- **Why Useful**: Patterns for memory visualization
- **For Clorus**: Debug mode design

---

## 8. Rust & Systems Programming

### 📄 "Ownership You Can Count On" - Reinking et al. (2020)
**Rust-like Ownership with RC**

- **PDF**: https://www.microsoft.com/en-us/research/publication/ownership-you-can-count-on/
- **Why Relevant**: Combining ownership and RC
- **For Clorus**: Future systems programming features

### 📄 Rust RFCs
**Design Decisions**

- **Repo**: https://github.com/rust-lang/rfcs
- **Key RFCs**:
  - RFC 1214: Clarify (and improve) rules for projections and well-formedness
  - RFC 2094: Non-lexical lifetimes
- **For Clorus**: Learn from Rust's experience

---

## Recommended Reading Order

### Week 1: Foundations
1. ✅ "Simple Made Easy" (video, 45 min) - Philosophy
2. ✅ "Ideal Hash Trees" (paper, 2-3 hours) - HAMT
3. ✅ Clojure source: `PersistentVector.java` (1 hour) - Implementation

### Week 2: Deep Dive
4. ✅ "Purely Functional Data Structures" (book, chapters 2-4) - Theory
5. ✅ "Unified Theory of Garbage Collection" (paper) - Memory management
6. ✅ Kaleidoscope Tutorial (tutorial) - LLVM practice

### Week 3: Advanced
7. ✅ "RRB-Trees" (paper) - Advanced vectors
8. ✅ "Perceus" (paper) - Modern RC
9. ✅ Clojure source: `PersistentHashMap.java` - HAMT implementation

---

## Quick Reference Guide

### For Persistent Vectors
**Primary**: Okasaki's thesis + Clojure source code
**Advanced**: RRB-Trees paper (future optimization)

### For Persistent HashMap
**Primary**: "Ideal Hash Trees" by Bagwell
**Reference**: Clojure's `PersistentHashMap.java`

### For Memory Management
**Start**: "Unified Theory of Garbage Collection"
**Modern RC**: Perceus paper, Lean 4 papers
**Decision**: RC vs GC document (we already wrote this!)

### For Language Design
**Philosophy**: "Simple Made Easy", "Value of Values"
**Practical**: Clojure source code, community discussions

### For LLVM
**Tutorial**: Kaleidoscope
**Reference**: LLVM Language Reference Manual

---

## Clorus-Specific Implementation Guide

### Phase 1: Persistent List (Simple!)
**References**:
- Okasaki Chapter 2: Lists
- Clojure's `PersistentList.java`

**Structure**:
```rust
struct PersistentList {
    head: Value,
    tail: *mut PersistentList,
    count: u64,
}
```

**Why Start Here**: Simplest persistent structure, tests RC

### Phase 2: Persistent Vector
**References**:
- Bagwell's HAMT paper (for understanding tries)
- Clojure's `PersistentVector.java` (for actual code)

**Key Insight from Papers**:
- 32-way branching (5 bits per level)
- Tail optimization (last 32 elements)
- Path copying for updates

### Phase 3: Persistent HashMap
**References**:
- Bagwell's "Ideal Hash Trees" (THE paper)
- Clojure's `PersistentHashMap.java`

**Key Insight from Paper**:
```
Bitmap trick:
  bitmap = 0b0000_0101_0001  (slots 0, 5, 8 occupied)
  To find array index for slot 5:
    count_bits_before(5) = popcount(bitmap & 0x1F) = 2
    array[2] = value_at_slot_5
```

---

## Papers NOT To Read (Yet)

### Too Advanced for Now
- ❌ Advanced type theory papers
- ❌ Formal verification papers
- ❌ Distributed systems papers
- ❌ Macro system theory

### Too Language-Specific
- ❌ JVM internals (we're using LLVM)
- ❌ JavaScript optimization papers
- ❌ Python-specific GC papers

---

## Keeping It Simple (à la Clojure)

From Rich Hickey's "Simple Made Easy":

**Simple**:
- One role, one task, one concept
- About lack of interleaving
- Objective

**Complex** (avoid!):
- Intertwined, braided together
- Multiple concepts bundled
- Hard to reason about

**For Clorus Implementation**:
```
✅ SIMPLE:
- Separate lexer, parser, codegen
- Pure persistent data structures
- Clear RC semantics
- One way to do things

❌ COMPLEX:
- Parser that also optimizes
- Mutable AND immutable structures
- RC + GC + manual management
- Multiple syntaxes for same thing
```

---

## Online Resources

### Clojure Community
- **ClojureVerse**: https://clojureverse.org/
- **Clojure Reddit**: r/Clojure
- **Slack**: clojurians.slack.com

### Courses
- **Purely Functional Data Structures**: Coursera (based on Okasaki)
- **Compilers**: Stanford CS143
- **LLVM Tutorial**: llvm.org/docs/tutorial

### Blogs
- **Rich Hickey's Talks**: https://github.com/matthiasn/talk-transcripts
- **The Morning Paper**: blog.acolyer.org (summaries of CS papers)

---

## TL;DR - Essential Reading List

**Must Read** (this week):
1. "Simple Made Easy" - Rich Hickey (video, 45 min)
2. "Ideal Hash Trees" - Phil Bagwell (paper, 3 hours)
3. Clojure source: PersistentVector.java (1 hour)

**Should Read** (this month):
4. "Purely Functional Data Structures" - Okasaki (book, selected chapters)
5. "Unified Theory of GC" - Bacon et al. (paper)
6. Kaleidoscope Tutorial (LLVM)

**Reference** (ongoing):
7. Clojure source code (when implementing features)
8. LLVM docs (when doing codegen)
9. Rust RFCs (for systems programming ideas)

---

## My Recommendation

**Start simple, iterate:**

1. **This Week**: Read "Ideal Hash Trees" + study Clojure's PersistentVector.java
2. **Implement**: Persistent List (simplest)
3. **Verify**: Use debug mode to check RC
4. **Then**: Vector → HashMap
5. **Future**: Optimize based on profiling

**Keep the Clojure spirit**:
- Data-oriented
- Immutable by default
- Simple (not easy)
- Pragmatic (not dogmatic)

Want me to start implementing? We can reference these papers as we go!
