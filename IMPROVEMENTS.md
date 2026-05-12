# IMPROVEMENTS

Este documento describe las 17 mejoras implementadas en el branch `feat/blockchain-improvements`.
El objetivo es elevar la calidad del código pasando de un prototipo funcional a una base sólida:
sin panics en producción, sin double-spending, con concurrencia correcta y con almacenamiento seguro de claves.

---

## Seguridad

### Cifrado de la wallet (AES-256-GCM)

**Problema:** El secret key de la wallet se guardaba en texto plano en `wallet.json`. Cualquier persona con acceso al archivo podía tomar control de los fondos.

**Solución:** El secret key ahora se cifra con AES-256-GCM antes de persistirse en disco. La clave de cifrado se deriva de una contraseña de usuario usando SHA-256. Se agregaron los métodos `Wallet::guardar_cifrado(path, password)` y `Wallet::cargar_cifrado(path, password)`. La contraseña se configura mediante la variable de entorno `WALLET_PASSWORD`.

**Archivos modificados:** `src/wallet.rs`

---

### Protección del endpoint `/wallet` contra sobrescritura

**Problema:** Cualquier llamada accidental al endpoint `POST /wallet` destruía la wallet existente de forma irreversible, sin advertencia ni confirmación.

**Solución:** El endpoint ahora verifica si `wallet.json` ya existe en disco antes de generar una nueva wallet. Si el archivo existe, responde con HTTP 409 Conflict sin modificar nada.

**Archivos modificados:** `src/bin/api.rs`, `src/main.rs`

---

### Verificación de firmas de transacciones en `validar()`

**Problema:** `Blockchain::validar()` solo verificaba la firma del bloque completo. Era posible inyectar transacciones con firmas inválidas o sin firmar dentro de un bloque cuya firma de cabecera fuera válida.

**Solución:** `validar()` ahora itera sobre cada transacción de cada bloque y verifica su firma individualmente antes de aceptar el bloque como válido.

**Archivos modificados:** `src/blockchain.rs`

---

## Corrección de bugs lógicos

### Double-spend prevention — balance tracking

**Problema:** No existía ningún control de saldo. Cualquier dirección podía emitir transacciones por cantidades arbitrarias sin tener fondos.

**Solución:** Se agregó `Blockchain::balance_of(pubkey)` que calcula el saldo disponible recorriendo tanto la cadena confirmada como el mempool pendiente. El método `add_transaction()` rechaza transacciones cuando el sender no tiene saldo suficiente.

**Archivos modificados:** `src/blockchain.rs`

---

### Soporte de transacciones coinbase (creación de moneda)

**Problema:** No había mecanismo para introducir moneda al sistema. Sin un origen de fondos, ninguna transacción podía tener saldo disponible.

**Solución:** Se agregó `Blockchain::add_coinbase(miner, reward)` que crea una transacción especial con `sender = [0u8; 32]`, acreditando fondos al minero. El endpoint `/mine` y el comando CLI `mine` agregan automáticamente una coinbase de 50 monedas al minar.

**Archivos modificados:** `src/blockchain.rs`

---

### CLI `send` ya no bypasea el mempool

**Problema:** El comando `send` de la CLI llamaba directamente a `add_block()`, creando bloques sin Proof of Work. Esto rompía la validez de la cadena y evitaba toda validación de transacciones.

**Solución:** El comando `send` ahora llama a `add_transaction()`, insertando la transacción en el mempool como corresponde. El bloque solo se crea cuando se ejecuta el mining.

**Archivos modificados:** `src/main.rs`

---

## Eliminación de panics en producción

### Fix `unwrap()` en verificación de firma del bloque

**Problema:** La conversión de bytes de firma en `validar()` usaba `unwrap()`. Si una firma tenía longitud incorrecta por datos corruptos, el servidor hacía panic y caía.

**Solución:** La conversión usa `try_into()` con manejo explícito del error: `Err(_) => return false`. El bloque se rechaza en lugar de terminar el proceso.

**Archivos modificados:** `src/blockchain.rs`

---

## Seguridad async y concurrencia

### Reemplazo de `std::sync::Mutex` por `tokio::sync::RwLock`

**Problema:** `std::sync::Mutex` puede causar deadlocks en contextos async si el lock se mantiene a través de un punto de suspensión (`.await`). El estado compartido de la API usaba ese tipo.

**Solución:** El estado compartido ahora usa `Arc<RwLock<Blockchain>>` de Tokio. Los handlers que solo leen usan `.read().await` y los que modifican usan `.write().await`, lo que además permite lecturas concurrentes.

**Archivos modificados:** `src/bin/api.rs`

---

### Mining no bloquea el servidor (`spawn_blocking`)

**Problema:** El handler de `/mine` mantenía el lock durante todo el Proof of Work, que puede tomar segundos. Durante ese tiempo, todas las demás requests quedaban bloqueadas.

**Solución:** El handler implementa tres fases separadas: (1) adquiere el write lock para extraer los datos necesarios y lo libera, (2) ejecuta el PoW en `tokio::task::spawn_blocking` sin ningún lock, (3) reacquiere el write lock solo para insertar el bloque resultante.

**Archivos modificados:** `src/bin/api.rs`

---

## Mejoras de diseño

### Anti-replay: nonce en transacciones

**Problema:** Sin nonce, una transacción válida podía ser copiada y re-incluida en múltiples bloques, debitando al sender más de una vez por la misma operación.

**Solución:** `Transaction` ahora incluye un campo `nonce: u64` generado aleatoriamente en cada construcción. El nonce forma parte del contenido firmado y del cálculo del Merkle root, haciendo que cada transacción sea única.

**Archivos modificados:** `src/transactions.rs`, `src/merkle.rs`

---

### Dificultad de minado configurable

**Problema:** La dificultad de minado estaba hardcodeada como `2` en el handler de la API, sin posibilidad de ajuste.

**Solución:** `Blockchain` almacena la dificultad en el struct (`difficulty: usize`, default `2`). Se agregó `new_blockchain_with_difficulty(n)` para instanciar con una dificultad específica. El método `minar()` ya no recibe el parámetro externamente; lo lee del struct.

**Archivos modificados:** `src/blockchain.rs`

---

### Nuevo comando CLI `mine`

**Problema:** La única forma de minar era a través de la API REST, lo que requería tener el servidor corriendo.

**Solución:** Se agregó el comando `mine` a la CLI. Al ejecutarlo, agrega una coinbase para el minero, mina el mempool actual con PoW, y persiste la blockchain actualizada en disco.

**Archivos modificados:** `src/cli.rs`, `src/main.rs`

---

### Métodos helper en `Blockchain` para separación de responsabilidades

**Problema:** El handler de mining necesitaba acceder a los internos del struct para extraer datos, minar y reinsertar el bloque, lo que obligaba a mantener el lock durante todo el proceso.

**Solución:** Se agregaron los métodos `tip()`, `take_mempool()`, `push_block()` y `difficulty()` a `Blockchain`. Estos permiten que el handler extraiga los datos necesarios, suelte el lock, mine sin él, y luego empuje el resultado adquiriendo el lock solo un instante.

**Archivos modificados:** `src/blockchain.rs`

---

## Calidad de código

### Typo corregido: `merklee_root` → `merkle_root`

**Problema:** El campo y las referencias al Merkle root tenían un typo (`merklee_root`) que causaba confusión al leer el código.

**Solución:** Renombrado a `merkle_root` en todos los usos.

**Archivos modificados:** `src/merkle.rs`

---

### Getters sin prefijo `get_`

**Problema:** Los métodos getter de `Block` y `Blockchain` usaban el prefijo `get_` (`get_hash()`, `get_index()`, `get_cadena()`), que es anti-idiomático en Rust.

**Solución:** Todos los getters fueron renombrados eliminando el prefijo: `get_hash()→hash()`, `get_index()→index()`, `get_cadena()→cadena()`, etc.

**Archivos modificados:** `src/block.rs`, `src/blockchain.rs`

---

### Derives combinados y field shorthand

**Problema:** Los derives estaban declarados en atributos separados (`#[derive(Debug)]` + `#[derive(Serialize, Deserialize)]`) y algunas inicializaciones de structs repetían el nombre de variable y campo innecesariamente.

**Solución:** Los derives se combinaron en un solo atributo (`#[derive(Debug, Serialize, Deserialize)]`) y se aplicó field shorthand donde el nombre de la variable coincide con el del campo.

**Archivos modificados:** `src/block.rs`, `src/transactions.rs`

---

### Tests con nombres descriptivos

**Problema:** Los tests tenían nombres genéricos (`mi_test`, `test_2`) que no comunicaban qué comportamiento estaban verificando.

**Solución:** Los tests fueron renombrados para describir la condición que prueban:
- `mi_test` → `hash_es_consistente_al_recalcular`
- `test_2` → `hash_cambia_al_agregar_transaccion`
- `mi_test` (blockchain) → `cadena_corrompida_no_es_valida`

**Archivos modificados:** `src/block.rs`, `src/blockchain.rs`

---

## Uso

### Contraseña de la wallet

La variable de entorno `WALLET_PASSWORD` protege el secret key en disco. Todos los comandos que interactúan con la wallet la requieren:

```bash
# Crear una nueva wallet cifrada
WALLET_PASSWORD=mi_password cargo run -- new-wallet

# Enviar una transacción (la wallet se descifra en memoria con la password)
WALLET_PASSWORD=mi_password cargo run -- send <destinatario> <cantidad>

# Minar desde la CLI
WALLET_PASSWORD=mi_password cargo run -- mine

# Levantar la API (la password se usa para descifrar la wallet al iniciar)
WALLET_PASSWORD=mi_password cargo run --bin api
```

Si `WALLET_PASSWORD` no está definida, los comandos que necesiten acceder a la wallet retornarán un error antes de intentar descifrar o guardar nada.
