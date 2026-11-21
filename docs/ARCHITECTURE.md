# Argento — Architettura

Argento è un'applicazione desktop per Linux dedicata al ritocco fotografico in
bianco e nero. Il modello di riferimento è Silver Efex: una libreria di preset
pronti all'uso, più regolazioni fini quando servono.

- **Linguaggio:** Rust
- **Interfaccia:** GTK4 + libadwaita
- **Piattaforma:** Linux (distribuzione prevista via Flatpak)

## Il nome

"Argento" richiama la stampa ai sali d'argento, il cuore della fotografia in
bianco e nero. Verifica di disponibilità (2026-11-21):

| Dove | Esito |
|---|---|
| crates.io | `argento` libero |
| Flathub | nessuna app omonima |
| Debian | nessun pacchetto omonimo |
| GitHub | nessun software fotografico rilevante |

I marchi registrati (EUIPO/USPTO) non sono stati verificati.

- **Crate e binario:** `argento`
- **App ID:** `io.github.enricod.Argento`

---

## 1. Struttura del workspace

```
argento/
├── Cargo.toml              # workspace
├── crates/
│   ├── argento-core/       # elaborazione pura, nessuna dipendenza GTK
│   ├── argento-io/         # caricamento, esportazione, colore, metadati, sidecar
│   └── argento-app/        # interfaccia GTK4 + libadwaita (binario "argento")
└── docs/
    └── ARCHITECTURE.md
```

**Dipendenze tra crate:**

```
argento-app ──► argento-io ──► argento-core
     └────────────────────────────►┘
```

`argento-core` non dipende da nulla del progetto e non conosce né GTK né il
file system. Così si può testare senza interfaccia, misurare con benchmark e
riusare in futuro (CLI, plugin per GIMP o darktable).

---

## 2. argento-core

### 2.1 Moduli

| Modulo | Responsabilità |
|---|---|
| `image` | Buffer `f32` lineari (RGB e luminanza a canale singolo), conversioni di spazio colore, ridimensionamento per il proxy di anteprima |
| `recipe` | La ricetta: tutti i parametri di un'elaborazione. Serializzabile (`serde`) e con numero di versione |
| `ops::filter` | Da RGB a luminanza con i filtri colorati (rosso, arancio, giallo, verde, blu) e la sensibilità del filtro |
| `ops::tone` | Luminosità, contrasto, contrasto morbido, amplificazione di bianchi e neri, curve |
| `ops::structure` | Contrasto locale multi-scala, con controllo separato per alte luci, mezzitoni e ombre |
| `ops::control_point` | Maschere stile U-Point (posizione + colore originale + raggio) e regolazioni locali |
| `ops::film` | Profili pellicola: risposta spettrale, curva tonale caratteristica, grana |
| `ops::toning` | Viraggi (seppia, selenio, cianotipia…) e split toning, da luminanza di nuovo a RGB |
| `ops::finishing` | Vignettatura, bordi bruciati, cornici |
| `pipeline` | Esecuzione della ricetta per stadi, con una cache per ogni stadio |
| `preset` | Preset integrati e preset dell'utente: nome, categoria, ricetta parziale |

### 2.2 La pipeline

L'ordine degli stadi è fisso, come in Silver Efex:

```
RGB sorgente ──┬─► [1] Filtro / pellicola (RGB→L)
               │          │
               │          ▼
               │   [2] Regolazioni globali (tono, struttura)
               │          │
               │          ▼
               └─► [3] Control point  ◄── usa il colore originale per le maschere
                          │
                          ▼
                   [4] Pellicola: curva tonale + grana
                          │
                          ▼
                   [5] Viraggio (L→RGB)
                          │
                          ▼
                   [6] Finitura (vignettatura, bordi, cornice)
                          │
                          ▼
                     RGB di uscita
```

Note:

- Gli stadi 1–4 lavorano su un'immagine di **luminanza a canale singolo**: un
  terzo dei dati rispetto all'RGB, quindi più veloce.
- Lo stadio 3 riceve anche l'**RGB sorgente**, perché le maschere U-Point
  selezionano le zone in base al colore originale.
- Se nella ricetta è scelta una pellicola, la sua risposta spettrale sostituisce
  il mix del filtro allo stadio 1.

### 2.3 Principi

1. **Operazioni pure.** Ogni stadio è una funzione
   `(input, parametri) → output`, senza stato nascosto. Ne derivano quasi
   gratis:
   - annulla/ripeti, come pila di ricette;
   - preset, come ricette parziali applicate sopra quella corrente;
   - sidecar, come ricetta salvata su file;
   - un futuro passaggio alla GPU, una funzione alla volta.

2. **Cache per stadio.** Ogni stadio conserva il proprio output insieme
   all'hash dei parametri che lo hanno prodotto. Quando cambia un parametro,
   si ricalcola solo da quello stadio in poi. Esempio: muovere la
   vignettatura (stadio 6) non ricalcola la struttura (stadio 2), che è lo
   stadio più costoso.

3. **Parametri in unità relative.** Raggi (struttura, control point,
   vignettatura) e dimensione della grana sono espressi in proporzione alla
   dimensione dell'immagine, non in pixel. Così l'anteprima su proxy e
   l'esportazione a piena risoluzione danno lo stesso risultato visivo.

4. **Spazio di lavoro lineare `f32`.** Tutta la pipeline lavora in RGB lineare
   a virgola mobile. La conversione verso lo spazio del monitor o del file
   avviene solo in uscita.

### 2.4 Ricetta (schizzo)

```rust
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Recipe {
    pub version: u32,
    pub filter: FilterParams,          // stadio 1
    pub film: Option<FilmParams>,      // stadi 1 e 4
    pub global: GlobalParams,          // stadio 2: tono + struttura
    pub control_points: Vec<ControlPoint>, // stadio 3
    pub toning: ToningParams,          // stadio 5
    pub finishing: FinishingParams,    // stadio 6
}
```

Un preset è una ricetta parziale: specifica solo i campi che vuole impostare e
lascia invariati gli altri (ad esempio i control point dell'utente).

---

## 3. argento-io

| Modulo | Responsabilità |
|---|---|
| `load` | JPEG, PNG, TIFF (anche a 16 bit). RAW in una fase successiva (`rawler` o `libraw`) |
| `color` | Profili ICC con `lcms2`: dall'input allo spazio di lavoro, dallo spazio di lavoro al monitor e al file di esportazione |
| `metadata` | Lettura e copia di EXIF/XMP all'esportazione (`rexiv2`) |
| `export` | JPEG, TIFF a 16 bit, PNG; ridimensionamento, qualità, profilo di uscita |
| `sidecar` | File `foto.jpg.argento` (TOML) accanto all'originale. L'originale non viene mai modificato |

---

## 4. argento-app

| Modulo | Responsabilità |
|---|---|
| `window` | Layout Adwaita: preset a sinistra, immagine al centro, regolazioni a destra |
| `canvas` | Visualizzazione con zoom e pan, confronto prima/dopo (affiancato o a tendina), sovrapposizione dei control point |
| `preset_browser` | Griglia di miniature dei preset, generate sulla foto corrente |
| `adjustments` | Pannelli di regolazione, uno per sezione della pipeline |
| `histogram` | Istogramma della luminanza |
| `history` | Annulla/ripeti come pila di ricette |
| `worker` | Thread di elaborazione (vedi sotto) |
| `export_dialog` | Opzioni di esportazione; elaborazione a piena risoluzione in background con avanzamento |

### 4.1 Modello di concorrenza

```
UI thread                         Worker thread
─────────                         ─────────────
slider cambia ──► invia Recipe ─► prende solo l'ULTIMA ricetta in coda
                                  (le richieste superate vengono scartate)
                                  pipeline.run(proxy, recipe)
aggiorna canvas ◄─ MemoryTexture ◄┘
```

- L'anteprima lavora su un **proxy** grande quanto il viewport (ad esempio
  2048 px sul lato lungo), non sull'immagine originale.
- Il worker unisce le richieste in arrivo: se mentre elabora arrivano cinque
  ricette, quando ha finito elabora solo l'ultima.
- Le miniature dei preset si generano su un proxy molto piccolo (circa 256 px),
  in un pool separato, così non rallentano l'anteprima principale.
- L'esportazione a piena risoluzione gira in un task separato e non blocca
  l'interfaccia.

---

## 5. Dipendenze principali previste

| Area | Crate |
|---|---|
| UI | `gtk4`, `libadwaita` |
| Immagini | `image` |
| Parallelismo | `rayon` |
| Colore | `lcms2` |
| Metadati | `rexiv2` |
| Serializzazione | `serde`, `toml` |
| RAW (in futuro) | `rawler` oppure binding a `libraw` |
| GPU (in futuro) | `wgpu` |

---

## 6. Decisioni

| # | Decisione | Motivazione |
|---|---|---|
| D1 | Elaborazione su CPU con `rayon`; GPU in futuro | Più semplice da sviluppare e da testare. Le operazioni pure permettono di portarle su GPU una alla volta |
| D2 | RAW esclusi dall'MVP | I file arrivano spesso già sviluppati da darktable o RawTherapee. Il supporto RAW è un progetto a sé |
| D3 | Control point nella versione 2 | Sono la parte più complessa, sia come algoritmo sia come interfaccia. Il modulo esiste nell'architettura fin dall'inizio |
| D4 | Spazio di lavoro RGB lineare `f32` | Precisione ed operazioni fisicamente corrette (mix dei canali, grana) |
| D5 | Pipeline a ordine fisso | Corrisponde al modello di Silver Efex, semplifica cache e interfaccia |
| D6 | Parametri spaziali in unità relative | Anteprima ed esportazione devono coincidere |
| D7 | Editing non distruttivo con sidecar TOML | L'originale non viene mai toccato; il sidecar è leggibile e versionabile |

---

## 7. Roadmap

### MVP
- Workspace e scheletro dell'applicazione GTK
- Apertura di JPEG, PNG e TIFF; gestione colore di base
- Pipeline con cache per stadio, worker di anteprima su proxy
- Stadio 1: filtri colorati
- Stadio 2: luminosità, contrasto, struttura
- 10–15 preset integrati con browser a miniature
- Confronto prima/dopo, annulla/ripeti, istogramma
- Sidecar, esportazione JPEG e TIFF

### Versione 2
- Pellicole: risposta spettrale, curve, grana
- Viraggi e split toning
- Finitura: vignettatura, bordi bruciati, cornici
- Curve manuali
- Control point
- Preset dell'utente (salvataggio e importazione)

### Versione 3
- Supporto RAW
- Accelerazione GPU
- Elaborazione in batch
- Pacchetto Flatpak e pubblicazione su Flathub
