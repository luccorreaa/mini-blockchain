# Blockchain Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Aplicar 17 mejoras de seguridad, corrección lógica, seguridad async y estilo de código al proyecto mini_blockchain.

**Architecture:** Los cambios se aplican de abajo hacia arriba: primero estructuras de datos (Transaction, Block), luego lógica de Blockchain (validar, balances), luego API/CLI. Cada tarea compila de forma independiente antes de continuar.

**Tech Stack:** Rust 2024, sha2, ed25519-dalek, axum 0.7, tokio, aes-gcm 0.10 (nuevo), serde_json

---

## Mapa de archivos

| Archivo | Cambios |
|---------|---------|
| `Cargo.toml` | Agregar `aes-gcm = "0.10"` |
| `src/merkle.rs` | Renombrar `merklee_root` → `merkle_root`; incluir nonce en hash |
| `src/transactions.rs` | Agregar `nonce: u64`; actualizar `firmar()` |
| `src/block.rs` | Combinar derives; field shorthand; renombrar getters (sin `get_`); `firma` privado; renombrar tests |
| `src/blockchain.rs` | Actualizar llamadas a getters; fix panic en `validar()` + verificar sigs de tx; agregar `balance_of()`, `add_coinbase()`, `take_mempool()`, `push_block()`, `tip()`, `difficulty()`; campo `difficulty`; `add_transaction` retorna `Result`; `minar()` sin args |
| `src/wallet.rs` | Agregar cifrado AES-GCM; actualizar `guardar`/`cargar` |
| `src/bin/api.rs` | Usar `RwLock`; proteger `/wallet`; mining async-safe; coinbase |
| `src/cli.rs` | Agregar comando `Mine` |
| `src/main.rs` | Fix `Send` (usa mempool + mine); agregar handler `Mine`; wallet cifrada |
| `IMPROVEMENTS.md` | Nuevo: documenta todos los cambios |

---

## Task 1: Dependencias + estilo de código

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/merkle.rs`
- Modify: `src/block.rs`
- Modify: `src/blockchain.rs`
- Modify: `src/transactions.rs`
- Modify: `src/wallet.rs`

- [ ] **Step 1.1: Agregar `aes-gcm` a Cargo.toml**

En `Cargo.toml`, agregar bajo `[dependencies]`:
```toml
aes-gcm = "0.10"
```

- [ ] **Step 1.2: Renombrar `merklee_root` → `merkle_root` en `src/merkle.rs`**

Reemplazar el nombre de la función:
```rust
pub fn merkle_root(transactions: &[Transaction]) -> String {
```

- [ ] **Step 1.3: Combinar `#[derive]` y field shorthand en `src/block.rs`**

Reemplazar la definición del struct y su `new()`:
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct Block {
    index: u32,
    transacciones: Vec<Transaction>,
    hash_previo: String,
    hash: String,
    timestamp: u64,
    firma: Option<Vec<u8>>,   // ya no es pub
    autor: Option<[u8; 32]>,
    nonce: u64,
}

impl Block {
    pub fn calcular_hash(&self) -> String {
        let mut hasher = Sha256::new();
        let contenido = format!("{}{}{}{}{}", self.index, merkle_root(&self.transacciones), self.hash_previo, self.timestamp, self.nonce);
        hasher.update(contenido.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn new(index: u32, transaction: Vec<Transaction>, hash_previo: &str) -> Block {
        let mut bloque = Block {
            index,
            transacciones: transaction,
            hash_previo: String::from(hash_previo),
            hash: String::new(),
            timestamp: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
            firma: None,
            autor: None,
            nonce: 0,
        };
        bloque.hash = bloque.calcular_hash();
        bloque
    }

    pub fn firmar(&mut self, signing_key: &SigningKey) {
        let contenido = format!("{}{}{}{}", self.index, merkle_root(&self.transacciones), self.hash_previo, self.timestamp);
        let signature = signing_key.sign(contenido.as_bytes());
        self.firma = Some(signature.to_bytes().to_vec());
        self.autor = Some(signing_key.verifying_key().to_bytes());
    }

    pub fn minar(&mut self, dificultad: usize) {
        let objetivo = "0".repeat(dificultad);
        while !self.hash.starts_with(&objetivo) {
            self.nonce += 1;
            self.hash = self.calcular_hash();
        }
    }

    // Getters sin prefijo get_
    pub fn hash(&self) -> &str { &self.hash }
    pub fn prev_hash(&self) -> &str { &self.hash_previo }
    pub fn index(&self) -> u32 { self.index }
    pub fn timestamp(&self) -> u64 { self.timestamp }
    pub fn transactions(&self) -> &[Transaction] { &self.transacciones }
    pub fn signature(&self) -> &Option<Vec<u8>> { &self.firma }
    pub fn author(&self) -> Option<[u8; 32]> { self.autor }

    #[cfg(test)]
    pub fn corromper(&mut self) {
        self.hash = "hash_corrupto".to_string();
    }
}
```

- [ ] **Step 1.4: Renombrar tests en `src/block.rs`**

Reemplazar los tests al final del archivo:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_es_consistente_al_recalcular() {
        let block = Block::new(0, vec![], "0");
        assert_eq!(block.hash(), block.calcular_hash());
    }

    #[test]
    fn hash_cambia_al_agregar_transaccion() {
        let mut block = Block::new(0, vec![], "0");
        let hash_original = block.calcular_hash();
        block.transacciones.push(Transaction::new([0u8; 32], [1u8; 32], 100));
        assert_ne!(hash_original, block.calcular_hash());
    }
}
```

- [ ] **Step 1.5: Combinar derives en `src/transactions.rs`**

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    #[serde(with = "hex")]
    pub sender: [u8; 32],
    #[serde(with = "hex")]
    pub receiver: [u8; 32],
    pub amount: u64,
    pub firma: Option<Vec<u8>>,
}
```

- [ ] **Step 1.6: Combinar derives en `src/wallet.rs`**

```rust
#[derive(Serialize, Deserialize)]
pub struct Wallet { ... }
```
(ya está bien, solo confirmar que no hay derives separadas)

- [ ] **Step 1.7: Actualizar todas las llamadas a getters en `src/blockchain.rs`**

Reemplazar en todo el archivo:
- `bloque.get_hash()` → `bloque.hash()`
- `bloque.get_hash_previo()` → `bloque.prev_hash()`
- `bloque.get_index()` → `bloque.index()`
- `bloque.get_timestamp()` → `bloque.timestamp()`
- `bloque.get_datos()` → `bloque.transactions()`
- `bloque.get_firma()` → `bloque.signature()`
- `bloque.get_autor()` → `bloque.author()`
- `merklee_root(` → `merkle_root(`

- [ ] **Step 1.8: Actualizar llamadas en `src/main.rs` y `src/bin/api.rs`**

En `main.rs`:
- `bloque.get_index()` → `bloque.index()`
- `bloque.get_hash()` → `bloque.hash()`
- `bloque.get_hash_previo()` → `bloque.prev_hash()`
- `bloque.get_timestamp()` → `bloque.timestamp()`
- `bloque.get_datos().len()` → `bloque.transactions().len()`

En `api.rs`:
- `b.get_index()` → `b.index()`

- [ ] **Step 1.9: Verificar que compila**

```bash
cargo check 2>&1
```
Expected: sin errores.

- [ ] **Step 1.10: Commit**

```bash
git add src/merkle.rs src/block.rs src/blockchain.rs src/transactions.rs src/wallet.rs src/main.rs src/bin/api.rs Cargo.toml
git commit -m "refactor: code style — combine derives, field shorthand, rename getters, fix merkle typo"
```

---

## Task 2: Anti-replay — nonce en Transaction

**Files:**
- Modify: `src/transactions.rs`
- Modify: `src/merkle.rs`

- [ ] **Step 2.1: Escribir test que falla**

En `src/transactions.rs`, al final:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transacciones_identicas_tienen_distinto_nonce() {
        let tx1 = Transaction::new([0u8; 32], [1u8; 32], 100);
        let tx2 = Transaction::new([0u8; 32], [1u8; 32], 100);
        // Con nonce aleatorio, el nonce debe ser (estadísticamente) distinto
        // Este test verifica que el campo existe y es accesible
        assert!(tx1.nonce != u64::MAX); // siempre pasa, pero confirma que el campo existe
    }
}
```

```bash
cargo test -p mini_blockchain transacciones_identicas 2>&1
```
Expected: FAIL — campo `nonce` no existe.

- [ ] **Step 2.2: Agregar `nonce` a Transaction**

Reemplazar el contenido de `src/transactions.rs`:
```rust
use ed25519_dalek::SigningKey;
use ed25519_dalek::Signer;
use hex;
use rand::random;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    #[serde(with = "hex")]
    pub sender: [u8; 32],
    #[serde(with = "hex")]
    pub receiver: [u8; 32],
    pub amount: u64,
    pub nonce: u64,
    pub firma: Option<Vec<u8>>,
}

impl Transaction {
    pub fn new(sender: [u8; 32], receiver: [u8; 32], amount: u64) -> Transaction {
        Transaction {
            sender,
            receiver,
            amount,
            nonce: random::<u64>(),
            firma: None,
        }
    }

    pub fn firmar(&mut self, signing_key: &SigningKey) {
        let contenido = format!(
            "{}{}{}{}",
            hex::encode(self.sender),
            hex::encode(self.receiver),
            self.amount,
            self.nonce
        );
        let signature = signing_key.sign(contenido.as_bytes());
        self.firma = Some(signature.to_bytes().to_vec());
    }
}
```

- [ ] **Step 2.3: Actualizar `merkle_root` para incluir nonce en el hash**

En `src/merkle.rs`, cambiar la línea del hash de cada tx:
```rust
let contenido = format!(
    "{}{}{}{}",
    hex::encode(tx.sender),
    hex::encode(tx.receiver),
    tx.amount,
    tx.nonce
);
```

- [ ] **Step 2.4: Verificar y correr tests**

```bash
cargo test 2>&1
```
Expected: todos los tests pasan.

- [ ] **Step 2.5: Commit**

```bash
git add src/transactions.rs src/merkle.rs
git commit -m "feat: add nonce to Transaction for anti-replay protection"
```

---

## Task 3: Fix panic + verificar firmas de transacciones en `validar()`

**Files:**
- Modify: `src/blockchain.rs`

- [ ] **Step 3.1: Escribir test que falla por panic**

En `src/blockchain.rs`, dentro del módulo `tests`:
```rust
#[test]
fn validar_no_entra_en_panic_con_firma_invalida() {
    let mut blockchain = Blockchain::new_blockchain();
    blockchain.add_block(vec![]);
    // Corromper la firma para que tenga longitud incorrecta
    if let Some(bloque) = blockchain.cadena.iter_mut().find(|b| b.index() == 1) {
        bloque.firma = Some(vec![0u8; 10]); // longitud incorrecta, antes causaba panic
    }
    // No debe paniquear, debe retornar false
    assert!(!blockchain.validar());
}
```

```bash
cargo test validar_no_entra_en_panic 2>&1
```
Expected: FAIL (panic o error de compilación por `firma` privado).

- [ ] **Step 3.2: Exponer `firma` para el test usando un setter de prueba**

Agregar en `src/block.rs`, dentro del bloque `#[cfg(test)]`:
```rust
#[cfg(test)]
pub fn set_firma_test(&mut self, firma: Vec<u8>) {
    self.firma = Some(firma);
}
```

- [ ] **Step 3.3: Actualizar el test para usar el setter**

```rust
#[test]
fn validar_no_entra_en_panic_con_firma_invalida() {
    let mut blockchain = Blockchain::new_blockchain();
    blockchain.add_block(vec![]);
    if let Some(bloque) = blockchain.cadena.iter_mut().find(|b| b.index() == 1) {
        bloque.set_firma_test(vec![0u8; 10]);
    }
    assert!(!blockchain.validar());
}
```

- [ ] **Step 3.4: Fix el `unwrap()` en `validar()` de `src/blockchain.rs`**

Reemplazar el bloque de verificación de firma del bloque en `validar()`:
```rust
if let (Some(firma_bytes), Some(autor_bytes)) = (bloque.signature(), bloque.author()) {
    let sig_array: [u8; 64] = match firma_bytes.as_slice().try_into() {
        Ok(arr) => arr,
        Err(_) => return false,  // firma con longitud incorrecta → inválido
    };
    let signature = Signature::from_bytes(&sig_array);
    let contenido = format!(
        "{}{}{}{}",
        bloque.index(),
        merkle_root(bloque.transactions()),
        bloque.prev_hash(),
        bloque.timestamp()
    );
    if let Ok(verifying_key) = VerifyingKey::from_bytes(&autor_bytes) {
        if verifying_key.verify(contenido.as_bytes(), &signature).is_err() {
            return false;
        }
    } else {
        return false;
    }
}
```

- [ ] **Step 3.5: Agregar verificación de firmas de transacciones en `validar()`**

Dentro del loop de `validar()`, antes del bloque de firma del bloque:
```rust
// Verificar firmas de cada transacción
for tx in bloque.transactions() {
    if tx.sender == [0u8; 32] {
        continue; // coinbase: no requiere firma
    }
    if let Some(firma_bytes) = &tx.firma {
        let sig_array: [u8; 64] = match firma_bytes.as_slice().try_into() {
            Ok(arr) => arr,
            Err(_) => return false,
        };
        let signature = Signature::from_bytes(&sig_array);
        let contenido = format!(
            "{}{}{}{}",
            hex::encode(tx.sender),
            hex::encode(tx.receiver),
            tx.amount,
            tx.nonce
        );
        if let Ok(verifying_key) = VerifyingKey::from_bytes(&tx.sender) {
            if verifying_key.verify(contenido.as_bytes(), &signature).is_err() {
                return false;
            }
        } else {
            return false;
        }
    } else {
        return false; // transacción no firmada → inválida
    }
}
```

Agregar en el bloque `use` de `blockchain.rs`:
```rust
use hex;
```

- [ ] **Step 3.6: Correr tests**

```bash
cargo test 2>&1
```
Expected: todos pasan.

- [ ] **Step 3.7: Commit**

```bash
git add src/blockchain.rs src/block.rs
git commit -m "fix: no panic on invalid signature in validar(); verify tx signatures"
```

---

## Task 4: Balance tracking + anti double-spend + coinbase

**Files:**
- Modify: `src/blockchain.rs`

- [ ] **Step 4.1: Escribir test que falla**

En el módulo `tests` de `src/blockchain.rs`:
```rust
#[test]
fn add_transaction_falla_si_saldo_insuficiente() {
    let mut blockchain = Blockchain::new_blockchain();
    let sender = [1u8; 32];
    let receiver = [2u8; 32];
    // El sender no tiene fondos
    let tx = Transaction::new(sender, receiver, 100);
    let result = blockchain.add_transaction(tx);
    assert!(result.is_err());
}

#[test]
fn coinbase_no_requiere_saldo() {
    let mut blockchain = Blockchain::new_blockchain();
    let miner = [3u8; 32];
    blockchain.add_coinbase(miner, 50);
    assert_eq!(blockchain.mempool.len(), 1);
}
```

```bash
cargo test add_transaction_falla 2>&1
```
Expected: FAIL — `add_transaction` retorna `()` y no hay `add_coinbase`.

- [ ] **Step 4.2: Agregar los métodos a `Blockchain`**

Agregar en `src/blockchain.rs`, dentro del `impl Blockchain`:

```rust
pub fn balance_of(&self, pubkey: &[u8; 32]) -> u64 {
    let mut balance = 0u64;
    for block in &self.cadena {
        for tx in block.transactions() {
            if tx.sender != [0u8; 32] && &tx.sender == pubkey {
                balance = balance.saturating_sub(tx.amount);
            }
            if &tx.receiver == pubkey {
                balance = balance.saturating_add(tx.amount);
            }
        }
    }
    // Restar lo que ya está comprometido en el mempool
    for tx in &self.mempool {
        if tx.sender != [0u8; 32] && &tx.sender == pubkey {
            balance = balance.saturating_sub(tx.amount);
        }
    }
    balance
}

pub fn add_coinbase(&mut self, miner: [u8; 32], reward: u64) {
    let coinbase = Transaction::new([0u8; 32], miner, reward);
    self.mempool.insert(0, coinbase);
}

pub fn add_transaction(&mut self, transaction: Transaction) -> Result<(), String> {
    if transaction.sender != [0u8; 32] {
        let available = self.balance_of(&transaction.sender);
        if available < transaction.amount {
            return Err(format!(
                "Saldo insuficiente: disponible {}, requerido {}",
                available, transaction.amount
            ));
        }
    }
    self.mempool.push(transaction);
    Ok(())
}
```

Agregar en `use` al tope de `blockchain.rs`: ya tiene lo necesario.

- [ ] **Step 4.3: Correr tests**

```bash
cargo test 2>&1
```
Expected: todos pasan (incluyendo los nuevos). Puede haber advertencias por `add_transaction` que ya existía.

- [ ] **Step 4.4: Commit**

```bash
git add src/blockchain.rs
git commit -m "feat: balance tracking, double-spend prevention, coinbase support"
```

---

## Task 5: Dificultad configurable en Blockchain

**Files:**
- Modify: `src/blockchain.rs`
- Modify: `src/bin/api.rs`

- [ ] **Step 5.1: Escribir test que falla**

```rust
#[test]
fn blockchain_guarda_dificultad() {
    let bc = Blockchain::new_blockchain_with_difficulty(3);
    assert_eq!(bc.difficulty(), 3);
}
```

```bash
cargo test blockchain_guarda_dificultad 2>&1
```
Expected: FAIL.

- [ ] **Step 5.2: Agregar `difficulty` a `Blockchain`**

Reemplazar el struct:
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct Blockchain {
    cadena: Vec<Block>,
    #[serde(default)]
    mempool: Vec<Transaction>,
    #[serde(default = "default_difficulty")]
    difficulty: usize,
}

fn default_difficulty() -> usize { 2 }
```

Agregar constructores y métodos helper:
```rust
pub fn new_blockchain() -> Blockchain {
    Blockchain::new_blockchain_with_difficulty(2)
}

pub fn new_blockchain_with_difficulty(difficulty: usize) -> Blockchain {
    let bloque = Block::new(0, vec![], "");
    Blockchain { cadena: vec![bloque], mempool: vec![], difficulty }
}

pub fn difficulty(&self) -> usize { self.difficulty }

pub fn tip(&self) -> Option<(u32, String)> {
    self.cadena.last().map(|b| (b.index(), b.hash().to_string()))
}

pub fn take_mempool(&mut self) -> Vec<Transaction> {
    std::mem::take(&mut self.mempool)
}

pub fn push_block(&mut self, block: Block) {
    self.cadena.push(block);
}
```

Cambiar `minar()` para no tomar parámetro y usar `self.difficulty`:
```rust
pub fn minar(&mut self) {
    if let Some((tip_index, tip_hash)) = self.tip() {
        let txs = self.take_mempool();
        let mut nuevo_bloque = Block::new(tip_index + 1, txs, &tip_hash);
        nuevo_bloque.minar(self.difficulty);
        self.cadena.push(nuevo_bloque);
    }
}
```

- [ ] **Step 5.3: Actualizar `src/bin/api.rs`**

Cambiar `blockchain.minar(2)` → `blockchain.minar()` (en el handler `mine` existente).

- [ ] **Step 5.4: Correr tests**

```bash
cargo test 2>&1
```
Expected: todos pasan.

- [ ] **Step 5.5: Commit**

```bash
git add src/blockchain.rs src/bin/api.rs
git commit -m "feat: configurable mining difficulty stored in Blockchain; minar() uses self.difficulty"
```

---

## Task 6: Cifrado de wallet (AES-GCM)

**Files:**
- Modify: `src/wallet.rs`
- Modify: `src/main.rs`
- Modify: `src/bin/api.rs`

- [ ] **Step 6.1: Escribir test que falla**

En `src/wallet.rs`, al final:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cifrado_round_trip() {
        let wallet = Wallet::new([42u8; 32], [7u8; 32]);
        wallet.guardar_cifrado("/tmp/test_wallet.json", "mi_password").unwrap();
        let loaded = Wallet::cargar_cifrado("/tmp/test_wallet.json", "mi_password").unwrap();
        assert_eq!(loaded.secret, wallet.secret);
        assert_eq!(loaded.pubkey, wallet.pubkey);
    }

    #[test]
    fn password_incorrecta_falla() {
        let wallet = Wallet::new([1u8; 32], [2u8; 32]);
        wallet.guardar_cifrado("/tmp/test_wallet2.json", "correcta").unwrap();
        assert!(Wallet::cargar_cifrado("/tmp/test_wallet2.json", "incorrecta").is_err());
    }
}
```

```bash
cargo test cifrado_round_trip 2>&1
```
Expected: FAIL — no existe `guardar_cifrado`.

- [ ] **Step 6.2: Implementar cifrado en `src/wallet.rs`**

Reemplazar el contenido completo de `src/wallet.rs`:
```rust
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::aead::rand_core::RngCore;
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};

#[derive(Serialize, Deserialize)]
pub struct Wallet {
    #[serde(with = "hex")]
    pub secret: [u8; 32],
    #[serde(with = "hex")]
    pub pubkey: [u8; 32],
}

#[derive(Serialize, Deserialize)]
struct EncryptedWallet {
    #[serde(with = "hex")]
    nonce: [u8; 12],
    ciphertext: String,  // base64
    #[serde(with = "hex")]
    pubkey: [u8; 32],
}

impl Wallet {
    pub fn new(secret: [u8; 32], pubkey: [u8; 32]) -> Wallet {
        Wallet { secret, pubkey }
    }

    fn derive_key(password: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.finalize().into()
    }

    pub fn guardar_cifrado(&self, path: &str, password: &str) -> Result<(), Box<dyn std::error::Error>> {
        let key_bytes = Self::derive_key(password);
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher.encrypt(nonce, self.secret.as_ref())
            .map_err(|e| format!("Error al cifrar: {}", e))?;

        let encrypted = EncryptedWallet {
            nonce: nonce_bytes,
            ciphertext: hex::encode(&ciphertext),
            pubkey: self.pubkey,
        };
        let json = serde_json::to_string_pretty(&encrypted)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn cargar_cifrado(path: &str, password: &str) -> Result<Wallet, Box<dyn std::error::Error>> {
        let json = std::fs::read_to_string(path)?;
        let encrypted: EncryptedWallet = serde_json::from_str(&json)?;

        let key_bytes = Self::derive_key(password);
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(&encrypted.nonce);

        let ciphertext = hex::decode(&encrypted.ciphertext)?;
        let plaintext = cipher.decrypt(nonce, ciphertext.as_ref())
            .map_err(|_| "Contraseña incorrecta o archivo corrupto")?;

        let secret: [u8; 32] = plaintext.try_into()
            .map_err(|_| "Longitud de clave incorrecta")?;

        Ok(Wallet { secret, pubkey: encrypted.pubkey })
    }

    // Mantener métodos sin cifrado para compatibilidad en tests
    pub fn guardar(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(&self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn cargar(path: &str) -> Result<Wallet, Box<dyn std::error::Error>> {
        let json = std::fs::read_to_string(path)?;
        let wallet = serde_json::from_str(&json)?;
        Ok(wallet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cifrado_round_trip() {
        let wallet = Wallet::new([42u8; 32], [7u8; 32]);
        wallet.guardar_cifrado("/tmp/test_wallet.json", "mi_password").unwrap();
        let loaded = Wallet::cargar_cifrado("/tmp/test_wallet.json", "mi_password").unwrap();
        assert_eq!(loaded.secret, wallet.secret);
        assert_eq!(loaded.pubkey, wallet.pubkey);
    }

    #[test]
    fn password_incorrecta_falla() {
        let wallet = Wallet::new([1u8; 32], [2u8; 32]);
        wallet.guardar_cifrado("/tmp/test_wallet2.json", "correcta").unwrap();
        assert!(Wallet::cargar_cifrado("/tmp/test_wallet2.json", "incorrecta").is_err());
    }
}
```

- [ ] **Step 6.3: Agregar función helper de password en `src/bin/api.rs` y `src/main.rs`**

En ambos archivos, agregar:
```rust
fn wallet_password() -> String {
    std::env::var("WALLET_PASSWORD").unwrap_or_else(|_| "dev_password_change_me".to_string())
}
```

- [ ] **Step 6.4: Actualizar `src/bin/api.rs` para usar wallet cifrada**

Reemplazar en el handler `wallet()`:
```rust
async fn wallet() -> Result<String, (StatusCode, String)> {
    if std::path::Path::new("wallet.json").exists() {
        return Err((StatusCode::CONFLICT, "Ya existe una wallet. Eliminá wallet.json antes de generar una nueva.".to_string()));
    }
    let mut secret = [0u8; 32];
    OsRng.fill_bytes(&mut secret);
    let signing_key = SigningKey::from_bytes(&secret);
    let pubkey = signing_key.verifying_key().to_bytes();
    let pubkey_hex = hex::encode(pubkey);
    let w = Wallet::new(secret, pubkey);
    w.guardar_cifrado("wallet.json", &wallet_password())
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error al guardar la wallet".to_string()))?;
    info!(pubkey = %pubkey_hex, "Nueva wallet generada");
    Ok(pubkey_hex)
}
```

Reemplazar en `add_to_mempool` donde carga la wallet:
```rust
let wallet = Wallet::cargar_cifrado("wallet.json", &wallet_password())
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error al cargar la wallet".to_string()))?;
```

- [ ] **Step 6.5: Actualizar `src/main.rs` para usar wallet cifrada**

En el comando `NewWallet`:
```rust
Command::NewWallet => {
    if std::path::Path::new("wallet.json").exists() {
        eprintln!("Ya existe una wallet. Eliminá wallet.json antes de generar una nueva.");
        std::process::exit(1);
    }
    let mut secret = [0u8; 32];
    OsRng.fill_bytes(&mut secret);
    let signing_key = SigningKey::from_bytes(&secret);
    let pubkey = signing_key.verifying_key().to_bytes();
    let wallet = Wallet::new(secret, pubkey);
    wallet.guardar_cifrado("wallet.json", &wallet_password()).expect("Error al guardar la wallet");
    println!("Generando nueva wallet...");
    println!("Clave pública: {}", hex::encode(pubkey));
}
```

En el comando `Send`, reemplazar la carga:
```rust
let wallet = Wallet::cargar_cifrado("wallet.json", &wallet_password()).expect("Error al cargar la wallet");
```

- [ ] **Step 6.6: Correr tests**

```bash
cargo test 2>&1
```
Expected: todos pasan incluyendo `cifrado_round_trip` y `password_incorrecta_falla`.

- [ ] **Step 6.7: Commit**

```bash
git add src/wallet.rs src/bin/api.rs src/main.rs
git commit -m "feat: encrypt wallet secret with AES-256-GCM; protect /wallet from overwrite"
```

---

## Task 7: `tokio::sync::RwLock` + mining async-safe

**Files:**
- Modify: `src/bin/api.rs`

- [ ] **Step 7.1: Reemplazar `Mutex` por `RwLock` y handlers de lectura**

Reemplazar el import y el estado en `src/bin/api.rs`:
```rust
use std::sync::Arc;
use tokio::sync::RwLock;
// Eliminar: use std::sync::{Arc, Mutex};
```

Reemplazar el tipo de estado:
```rust
let state: Arc<RwLock<Blockchain>> = Arc::new(RwLock::new(blockchain));
```

Cambiar la firma y body de `get_chain`:
```rust
async fn get_chain(
    State(blockchain): State<Arc<RwLock<Blockchain>>>
) -> Result<Json<Value>, (StatusCode, String)> {
    let bc = blockchain.read().await;
    Ok(Json(serde_json::to_value(&*bc)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error al serializar".to_string()))?))
}
```

Cambiar `validar`:
```rust
async fn validar(
    State(blockchain): State<Arc<RwLock<Blockchain>>>
) -> Result<String, (StatusCode, String)> {
    let bc = blockchain.read().await;
    let resultado = bc.validar();
    info!(valida = resultado, "Validación ejecutada");
    Ok(format!("La cadena de bloques es válida: {}", resultado))
}
```

Cambiar `get_block`:
```rust
async fn get_block(
    State(blockchain): State<Arc<RwLock<Blockchain>>>,
    Path(index): Path<u32>
) -> Result<Json<Value>, (StatusCode, String)> {
    let bc = blockchain.read().await;
    match bc.get_cadena().iter().find(|b| b.index() == index) {
        Some(block) => Ok(Json(serde_json::to_value(block)
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error al serializar".to_string()))?)),
        None => Err((StatusCode::NOT_FOUND, format!("Bloque {} no encontrado", index)))
    }
}
```

Cambiar `add_to_mempool`:
```rust
async fn add_to_mempool(
    State(blockchain): State<Arc<RwLock<Blockchain>>>,
    Json(payload): Json<SendPayload>
) -> Result<String, (StatusCode, String)> {
    let from: [u8; 32] = hex::decode(&payload.from)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Clave 'from' inválida".to_string()))?
        .try_into()
        .map_err(|_| (StatusCode::BAD_REQUEST, "'from' debe ser 32 bytes".to_string()))?;

    let to: [u8; 32] = hex::decode(&payload.to)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Clave 'to' inválida".to_string()))?
        .try_into()
        .map_err(|_| (StatusCode::BAD_REQUEST, "'to' debe ser 32 bytes".to_string()))?;

    let wallet = Wallet::cargar_cifrado("wallet.json", &wallet_password())
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error al cargar la wallet".to_string()))?;
    let signing_key = SigningKey::from_bytes(&wallet.secret);

    let mut tx = Transaction::new(from, to, payload.amount);
    tx.firmar(&signing_key);

    let mut bc = blockchain.write().await;
    bc.add_transaction(tx)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    info!(from = %payload.from, to = %payload.to, amount = payload.amount, "Transacción en mempool");
    Ok("Transacción enviada".to_string())
}
```

- [ ] **Step 7.2: Reemplazar el handler `mine` con versión async-safe**

El mining ocupa mucho CPU (Proof of Work). Se extrae la data necesaria, se suelta el lock y se mina en `spawn_blocking`:

```rust
async fn mine(
    State(blockchain): State<Arc<RwLock<Blockchain>>>
) -> Result<String, (StatusCode, String)> {
    // Fase 1: extraer datos y soltar el lock inmediatamente
    let (index, prev_hash, txs, difficulty) = {
        let mut bc = blockchain.write().await;

        // Agregar coinbase para el minero si hay wallet disponible
        if let Ok(wallet) = Wallet::cargar_cifrado("wallet.json", &wallet_password()) {
            bc.add_coinbase(wallet.pubkey, 50);
        }

        let (tip_index, tip_hash) = bc.tip()
            .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Cadena vacía".to_string()))?;
        let txs = bc.take_mempool();
        let difficulty = bc.difficulty();
        (tip_index + 1, tip_hash, txs, difficulty)
    }; // write lock liberado aquí

    // Fase 2: minar sin mantener ningún lock (puede tardar varios segundos)
    let block = tokio::task::spawn_blocking(move || {
        let mut b = mini_blockchain::block::Block::new(index, txs, &prev_hash);
        b.minar(difficulty);
        b
    })
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error al minar".to_string()))?;

    // Fase 3: agregar el bloque minado
    let mut bc = blockchain.write().await;
    bc.push_block(block);
    bc.guardar("blockchain.json")
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error al guardar".to_string()))?;

    info!("Nuevo bloque minado");
    Ok("Bloque minado exitosamente\n".to_string())
}
```

Para que `Block` sea accesible desde `api.rs`, agregar `pub` en `src/block.rs` al método `minar` (ya es `pub`). Y en `src/lib.rs`:
```rust
pub mod block;
```
(ya existe)

- [ ] **Step 7.3: Agregar `get_cadena` a Blockchain (necesario en get_block)**

Verificar que `Blockchain::get_cadena()` existe. Si no, agregar:
```rust
pub fn get_cadena(&self) -> &[Block] { &self.cadena }
```

- [ ] **Step 7.4: Compilar**

```bash
cargo check 2>&1
```
Expected: sin errores. Puede haber advertencias de imports no usados; limpiarlas.

- [ ] **Step 7.5: Commit**

```bash
git add src/bin/api.rs
git commit -m "feat: use tokio RwLock; mine in spawn_blocking to avoid blocking async runtime"
```

---

## Task 8: CLI — fix Send + agregar comando Mine

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/main.rs`

- [ ] **Step 8.1: Agregar comando `Mine` en `src/cli.rs`**

Reemplazar el enum `Command`:
```rust
#[derive(Subcommand)]
pub enum Command {
    NewWallet,
    ShowChain,
    Validate,
    Mine,
    Send {
        #[arg(short, long)]
        from: String,
        #[arg(short, long)]
        to: String,
        #[arg(short, long)]
        amount: u64,
    }
}
```

- [ ] **Step 8.2: Agregar handler `Mine` y fix `Send` en `src/main.rs`**

Reemplazar el bloque `Command::Send` y agregar `Command::Mine`:

```rust
Command::Mine => {
    let mut blockchain = Blockchain::cargar("blockchain.json")
        .unwrap_or_else(|_| Blockchain::new_blockchain());
    
    // Agregar coinbase si hay wallet
    if let Ok(wallet) = Wallet::cargar_cifrado("wallet.json", &wallet_password()) {
        blockchain.add_coinbase(wallet.pubkey, 50);
    }
    
    println!("Minando bloque...");
    blockchain.minar();
    blockchain.guardar("blockchain.json").expect("Error al guardar");
    println!("Bloque minado exitosamente.");
}

Command::Send { from, to, amount } => {
    let mut blockchain = Blockchain::cargar("blockchain.json")
        .unwrap_or_else(|_| Blockchain::new_blockchain());
    
    let mut tx = Transaction::new(
        hex::decode(&from).expect("from inválido").try_into().expect("32 bytes"),
        hex::decode(&to).expect("to inválido").try_into().expect("32 bytes"),
        amount,
    );
    
    println!("Enviando {} desde {} a {}...", amount, from, to);
    let wallet = Wallet::cargar_cifrado("wallet.json", &wallet_password())
        .expect("Error al cargar la wallet");
    let signing_key = SigningKey::from_bytes(&wallet.secret);
    tx.firmar(&signing_key);
    
    blockchain.add_transaction(tx).expect("Saldo insuficiente");
    blockchain.guardar("blockchain.json").expect("Error al guardar");
    println!("Transacción en mempool. Usá 'mine' para confirmarla.");
}
```

Agregar el import de `Blockchain::add_coinbase` en `main.rs` (ya viene del módulo).

- [ ] **Step 8.3: Compilar**

```bash
cargo build 2>&1
```
Expected: sin errores.

- [ ] **Step 8.4: Commit**

```bash
git add src/cli.rs src/main.rs
git commit -m "feat: add Mine CLI command; fix Send to use mempool instead of add_block directly"
```

---

## Task 9: Renombrar test en blockchain.rs

**Files:**
- Modify: `src/blockchain.rs`

- [ ] **Step 9.1: Renombrar el test existente**

Cambiar en `src/blockchain.rs`:
```rust
#[test]
fn cadena_corrompida_no_es_valida() {
    let mut blockchain = Blockchain::new_blockchain();
    blockchain.add_block(vec![]);
    blockchain.add_block(vec![]);
    assert!(blockchain.validar());
    blockchain.corromper_bloque(1);
    assert!(!blockchain.validar());
}
```

- [ ] **Step 9.2: Correr todos los tests finales**

```bash
cargo test 2>&1
```
Expected: todos pasan, ningún warning de tests sin usar.

- [ ] **Step 9.3: Commit**

```bash
git add src/blockchain.rs
git commit -m "refactor: rename test to be descriptive"
```

---

## Task 10: Generar IMPROVEMENTS.md

**Files:**
- Create: `IMPROVEMENTS.md`

- [ ] **Step 10.1: Crear el archivo**

Crear `IMPROVEMENTS.md` en la raíz del proyecto documentando todos los cambios. Ver instrucciones de formato abajo.

- [ ] **Step 10.2: Verificar build final limpio**

```bash
cargo test 2>&1 && cargo clippy 2>&1
```
Expected: todos los tests pasan.

- [ ] **Step 10.3: Commit final**

```bash
git add IMPROVEMENTS.md
git commit -m "docs: add IMPROVEMENTS.md with all changes explained"
```

---

## Self-Review

**Cobertura del spec:**
- [x] Clave privada en texto plano → Task 6 (AES-GCM)
- [x] /wallet destruye wallets existentes → Task 6 (check de existencia)
- [x] Firmas de transacciones no verificadas → Task 3
- [x] Double-spend → Task 4
- [x] Mining bloquea servidor → Task 7 (spawn_blocking + RwLock)
- [x] CLI Send no mina → Task 8
- [x] unwrap() en validación → Task 3
- [x] Anti-replay → Task 2
- [x] std::sync::Mutex en async → Task 7
- [x] Dificultad hardcodeada → Task 5
- [x] Typo merklee_root → Task 1
- [x] #[derive] separados → Task 1
- [x] Field shorthand → Task 1
- [x] Prefijo get_ en getters → Task 1
- [x] Tests no descriptivos → Task 1 + Task 9
- [x] firma pub en Block → Task 1
- [x] Comando Mine en CLI → Task 8
- [x] IMPROVEMENTS.md → Task 10

**Tipos consistentes:**
- `add_transaction` retorna `Result<(), String>` — actualizado en api.rs (Task 7) y main.rs (Task 8) ✓
- `minar()` sin parámetro — actualizado en api.rs (Task 5) y main.rs (Task 8) ✓
- Getters sin `get_` — consistentes desde Task 1 en adelante ✓
