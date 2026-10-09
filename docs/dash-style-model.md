# Modello mentale del dash (proposta)

**Stato:** proposta di design — descrive come *dovrebbe* comportarsi il tratteggio
lungo le linee, rispetto al motore attuale (`crates/burin-map-core/src/dashes.rs`
e parametri `dash_*` / `mod_*` in `style.rs`). Non è ancora implementato come
API o come preset.

## Perché questo documento

Chi usa Burin Map guarda l’output (linee più o meno piene in luce e ombra,
tratti regolari o “a mano”) e tende a ragionare così:

1. **Quanto inchiostro** lungo la linea in zone chiare vs scure.
2. **Ritmo regolare** (tratto e vuoto della stessa lunghezza quando è al 50%).
3. **Irregolarità** sullo stesso rapporto (stessa percentuale media, pattern meno
   meccanico).
4. **Eccezioni sul piano** (un po’ di caos dove il terreno è piatto, per non
   sembrare una stampa piatta).

Il motore oggi *si avvicina* a questa idea, ma i nomi e i legami tra parametri
non la raccontano in modo lineare. Questo file fissa il **desiderio** per una
rifattorizzazione futura (parametri, documentazione, preset Contours).

## Comportamento desiderato (solo dash)

### Livello 1 — Proporzione inchiostro (luce / ombra)

- Due estremi (o più, se servisse) che definiscono la **frazione di inchiostro
  lungo ogni singola linea tracciata**, interpolata tra “bianco” e “nero” nel
  senso del tono di rilievo locale.
- Esempio mentale: al **50%** in luce → trattini **regolari**, metà periodo
  inchiostrato e metà vuoto (stessa lunghezza tratto e gap).
- Con **100%** in luce **e** **100%** in ombra → linea **continua** lungo quel
  percorso (senza sorprese aggiuntive: niente lotterie che annullano il “pieno”).

**Corrispondenza attuale (approssimativa):** `dash_light`, `dash_dark`, più il
tono `t` letto dal campo `input_*` / `sun_*`.

**Gap attuale:** con `dash_light` e `dash_dark` a 1 la linea non è continua
finché `dash_drop` (default 1) e altri `mod_*` restano attivi; il motore non
usa la scorciatoia “polilinea intera” se `dash_drop > 0`.

### Livello 2 — Periodo regolare

- Un solo parametro base: **lunghezza del ciclo** tratto+vuoto lungo la linea
  (oggi: `dash_period` × `space`).
- A jitter zero e senza eccezioni, il pattern è **meccanico**: stesso passo,
  stesso rapporto tratto/vuoto fissato dal livello 1.

**Desiderio:** `dash_period` (o equivalente) definisce il **ritmo ideale
regolare**. Non dovrebbe competere con un secondo “asse” parallelo per la stessa
cosa.

**Gap attuale:** `mod_jitter` modifica già il periodo nello stesso loop del
duty; concettualmente l’irregolarità del periodo dovrebbe essere *derivata* dal
periodo base, non mescolata al livello 1.

### Livello 3 — Irregolarità del tratteggio

- Stessa **percentuale media** di inchiostro del livello 1, ma **spezzatura
  irregolare** rispetto al 50/50 meccanico (lunghezze e passi che variano).
- Un solo controllo (o una famiglia chiara) per “quanto è irregolare il
  tratteggio”, **dopo** aver fissato periodo regolare e duty.

**Corrispondenza attuale:** soprattutto `mod_jitter` (che oggi agisce anche su
periodo e lunghezza del tratto, e si attenua in ombra). Su linee già
**continue** (duty 1, drop 0, scorciatoia attiva) il jitter **non deve**
cambiare nulla — non ha senso “jitterare” una linea già piena.

### Livello 4 — Eccezioni sul terreno piatto / chiaro

- Separato dal “dash ideale”: sulle **pianure / zone molto chiare**, opzionale
  **lotteria** che alleggerisce o rompe il pattern perché un piano uniforme al
  100% meccanico è noioso.
- Nome concettuale tipo “floor / pianura”: non è la stessa cosa del cutoff.

**Corrispondenza attuale:**

| Desiderio | Parametro attuale | Nota |
|-----------|-------------------|------|
| Troppo chiaro → niente inchiostro | `dash_cutoff` | Regola **fissa**, non lotteria |
| Pianura → un po’ a caso | `dash_drop` | Forza del sorteggio; fascia legata anche a `input_flat` |
| Tono “piatto” nel campo | `input_flat` | Input del rilievo, non solo dash |

**Gap attuale:** `dash_drop` non è solo “lotteria pianura”; con valore alto
impedisce anche il percorso “linea continua” anche quando duty è 1.

### Fuori dal dash (stesso batch, altro gruppo)

- **`mod_breaks`**, **`mod_wobble`**, wave, smooth, blur, ecc. → **carattere
  della linea** (spezzare, uscire dal percorso, arrotondare il terreno che si
  segue). Stessa famiglia `mod_*` nel codice; **non** fanno parte del “dash
  ideale” ma possono interagire con l’output (es. breaks che tagliano un tratto
  lungo).
- **`cap`**, **`batch_stroke`** → aspetto del tratto **disegnato** (estremi
  tondi vs netti), non la regola tratto/vuoto.
- **`space`**, **`space_light`**, **`space_dark`** → distanza **tra linee
  parallele**, non lungo la linea.

## Riepilogo: modello a strati desiderato

```
tono (input/sun) ──► duty luce/ombra (%)     ← dash_light / dash_dark
                         │
periodo base regolare ──┼──► ciclo tratto+vuoto   ← dash_period (× space)
                         │
irregolarità tratteggio ┘   (opzionale)          ← oggi mod_jitter; idealmente legato al periodo
                         │
eccezione pianura/chiaro (opzionale)             ← oggi dash_drop (+ input_flat); cutoff = taglio netto
                         │
carattere linea (mod_*)                            ← breaks, wobble, …
```

## Obiettivi per una futura modifica

1. **Semantica chiara:** `dash_light` / `dash_dark` = percentuale lungo la
   linea; 1+1 con drop a 0 = continuo, senza parametri nascosti.
2. **Periodo:** un knob “regolare”; irregolarità del periodo come derivato
   esplicito (non duplicato concettualmente con jitter sparso).
3. **Drop / cutoff:** documentati come livelli distinti (taglio vs lotteria
   pianura), non come “altro dash”.
4. **Preset Contours:** valori default che riflettono l’effetto incisione *o*
   un preset “meccanico” con drop 0 per chi vuole il modello semplice.
5. **SYNTAX.md / `burin.parameters()`:** testi di help allineati al modello a
   strati, non solo ai nomi storici.

## Riferimenti nel codice attuale

- Spezzatura e duty: `crates/burin-map-core/src/dashes.rs`
- Definizioni e default: `crates/burin-map-core/src/style.rs` (macro `style!`)
- Discussione utente / conferme su `dash_drop`: conversazione di design, 2026

## Non in scope (qui)

- Spacing tra linee (`space_*`).
- Ombreggiatura (`input_*`, `sun_*`) se non come sorgente del tono per il dash.
- Acqua (`water_*`), shoreline, land-edge fray (vedi
  [land-edge-fray.md](land-edge-fray.md)).
