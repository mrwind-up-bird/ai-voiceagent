//! nyxCore integration — Persona Studio + Axiom (RAG) clients.
//!
//! Sub-Project E. The two services are independent and consumed
//! voluntarily by the user (Settings opt-in plus per-action invoke).
//! Both auth via Bearer tokens stored in the OS keychain (Sub-Project D):
//!   - `persona_studio` token for POST /api/v1/persona/chat + /persona/list
//!   - `nyxcore_axiom`  token for POST /api/v1/rag/search

pub mod client;
pub mod persona;
pub mod axiom;

pub use persona::*;
pub use axiom::*;
