# Modello mentale dash e spacing (proposta)

**Stato:** proposta di design — come *dovrebbero* leggersi **tratteggio lungo la
linea** e **distanza tra linee parallele**, rispetto al motore attuale
(`dashes.rs`, `fields.rs`, `style.rs`). Non è ancora implementato come API unificata
né come preset. Questo file serve anche da **quadro unico** per modifiche
future, così dash e spacing non vengono rifattorizzati in modo incompatibile.

## Perché questo documento

Chi usa Burin Map guarda l’output (linee più o meno piene in luce e ombra,
tratti regolari o “a mano”) e tende a ragionare così:

1. **Quanto inchiostro** lungo la linea in zone chiare vs scure.
2. **Ritmo regolare** (tratto e vuoto della stessa lunghezza quando è al 50%).
3. **Irregolarità** sullo stesso rapporto (stessa percentuale media, pattern meno
   meccanico).
4. **Eccezioni sul piano** (un po’ di caos dove il terreno è piatto, per non
   sembrare una stampa piatta).
5. **Quantizzazione dell’input** (opzionale): invece di interpolare sempre in
   modo continuo tra luce e ombra, poter **schiacciare il tono in pochi stati**
   (es. due) e ottenere zone con dash **meccanico e netto** — metà continue,
   metà tratteggiate — senza jitter o altri modificatori di irregolarità.

Il motore oggi *si avvicina* a questa idea, ma i nomi e i legami tra parametri
non la raccontano in modo lineare. Questo file fissa il **desiderio** per una
rifattorizzazione futura (parametri, documentazione, preset Contours).

## Due assi: perché oggi confondono

| | **Tra le linee** (spacing) | **Lungo la linea** (dash) |
|---|---------------------------|---------------------------|
| **Cosa controlla** | Distanza tra streamlines parallele | Quanto ogni linea è piena vs vuota, e il passo del motivo |
| **Modello atteso (utente)** | Valori in **px** in luce e in ombra | Percentuale / periodo in **px** (o multipli chiari del periodo) |
| **Modello attuale (motore)** | `space` (px) × moltiplicatore da `space_light` → `space_dark` (e `space_steep`, blur) | `dash_light` / `dash_dark` (0–1 duty); periodo = `dash_period` × **`space`** |

Quindi non è che il dash sia del tutto “assoluto” oggi: il **periodo del tratto**
è ancora legato al `space` di base (`dashes.rs`). Però **semanticamente** light/dark
del dash sono **estremi del duty**, mentre light/dark dello spacing sono
**moltiplicatori**, non “5 px in luce, 3 px in ombra”. È quella asimmetria che
fa pensare: “qui relativo, lì quasi assoluto”.

## Comportamento desiderato (dash lungo la linea)

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

### Livello opzionale — Quantizzazione dell’input (pochi stati)

Oggi il tono locale `t` che guida il dash è in pratica un **campo continuo**
(ombreggiatura + parametri `input_*`). Il duty diventa una **slide** tra
`dash_light` e `dash_dark` lungo tutti i valori intermedi.

**Desiderio:** poter **quantizzare** quell’input (o un derivato) prima che
entri nel dash, così da avere **N stati discreti** invece di una rampa infinita.

Esempio con **due stati** (caso tipico):

| Stato (dopo quantizzazione) | Aspetto lungo la linea | Parametri concettuali |
|----------------------------|------------------------|------------------------|
| “Chiaro” / sotto soglia | Tratteggio **regolare**, duty fisso (es. **0.5**, **0.2**, **0.1**) — trattino e vuoto della stessa lunghezza se duty = 0.5 | `dash_light` o duty dedicato allo stato chiaro |
| “Scuro” / sopra soglia | Linea **continua** (duty 1) | `dash_dark` = 1 (o stato “pieno”) |

Con **jitter a 0**, **breaks a 0**, **drop a 0**, il risultato non deve
sembrare “irregolare”: le aree quantizzate come scure sono **piene**, quelle
quantizzate come chiare sono **tratteggiate meccaniche** al rapporto scelto
(0.5 = metà-m metà vuoto, 0.2 = venti per cento inchiostro, ecc.). La
**frontiera** tra i due stati segue la soglia di quantizzazione sul rilievo, non
un sorteggio a caso.

**Perché è utile:** stile cartografico “a due inchiostri” (pieno in ombra,
trama regolare in luce) senza dover simulare il contrasto solo con
`input_gain` / gamma e senza usare `mod_jitter` per ottenere separazione visiva.

**Non è la stessa cosa di:**

- **`dash_drop`** — lotteria sul piano, non soglia netta su tutta l’area.
- **Interpolazione continua light/dark** — produce sempre gradienti di tratteggio,
  mai “metà mappa continua e metà a duty 0.5 fissi” se non per coincidenza.

**Implementazione (solo indicazioni, non esiste ancora):**

- Parametri tipo: numero di livelli (2, 3, …), soglie o step sul tono
  quantizzato, duty (o light/dark) **per livello**.
- La quantizzazione agisce **prima** del livello 1 del dash; periodo regolare
  (`dash_period`) resta il ritmo del tratteggio negli stati non pieni.

**Gap attuale:** nessuna quantizzazione esplicita del tono per il dash; solo
campo continuo + eventuale lottery (`dash_drop`) e taglio (`dash_cutoff`).

### Fuori dal dash (stesso batch, altro gruppo)

- **`mod_breaks`**, **`mod_wobble`**, wave, smooth, blur, ecc. → **carattere
  della linea** (spezzare, uscire dal percorso, arrotondare il terreno che si
  segue). Stessa famiglia `mod_*` nel codice; **non** fanno parte del “dash
  ideale” ma possono interagire con l’output (es. breaks che tagliano un tratto
  lungo).
- **`cap`**, **`batch_stroke`** → aspetto del tratto **disegnato** (estremi
  tondi vs netti), non la regola tratto/vuoto.
## Comportamento desiderato (spacing tra le linee)

### Modello a estremi in px (proposta)

Come per il dash, l’utente vorrebbe definire il **range** con due valori
**attesi in pixel** (o in unità di output map), non un “riferimento” più due
moltiplicatori:

- **`space_light`** → distanza tra linee sulle **parti illuminate** (es. 8 px).
- **`space_dark`** → distanza sulle **parti in ombra** (es. 4 px, più fitte).

Il tono locale (stesso campo che guida il dash) **interpola** tra questi due
estremi — eventualmente con la stessa **quantizzazione** a due stati del dash,
se si vuole coerenza cartografica.

**Oggi nel motore** (`fields.rs`): a ogni cella
`spacing_px = space × k`, con `k` da `space_light`…`space_dark` (moltiplicatori)
× `space_steep` × blur. I default (es. `space` 4.8, `space_light` 1.0,
`space_dark` 0.7) vanno letti come “4.8 px in luce, ~3.4 px in ombra”, non come
numeri diretti nel file di stile.

### Ha senso passare allo spacing “assoluto”?

**Sì, per coerenza del modello mentale** — stesso schema “estremo luce / estremo
ombra (+ opzionale quantizzazione)” per **due domande diverse**:

1. Quanto sono **lontane** le linee? → `space_light` / `space_dark` in px.
2. Quanto ogni linea è **piena**? → `dash_light` / `dash_dark` (duty).

Vantaggi:

- Preset e documentazione leggibili senza moltiplicare a mano.
- Allineamento con quantizzazione a due stati (soglia unica sul tono → spacing
  e dash saltano insieme agli stessi estremi).
- Meno rischio di “ho alzato `space` e ho cambiato anche il tratteggio” perché
  oggi `dash_period` scala con `space`.

Rischi / da progettare insieme al dash:

- **`space_steep`** (e gamma): oggi è un modificatore sulla densità in pendio;
  resta come terzo fattore o diventa parte del campo interpolato?
- **Acqua / shore** (`water.rs`): molte formule usano `space × mul`; vanno
  migrate in parallelo.
- **`dash_period`**: se spacing e dash si separano semanticamente, il periodo
  lungo la linea dovrebbe essere **in px** (o multiplo esplicito di
  `space_light`/`space_dark`), non agganciato al vecchio `space` unico.
- **Compatibilità**: alias o conversione dai vecchi `space` + moltiplicatori per
  non rompere script e preset.

**Conclusione:** includere la richiesta **nello stesso documento / stesso
refactor** ha senso: è un unico “linguaggio luce–ombra” per la trama, non due
feature indipendenti.

## Riepilogo: modello a strati desiderato

```
tono (input/sun) ──► [quantizzazione opzionale: 2+ stati]
                         │
           ┌─────────────┴─────────────┐
           ▼                           ▼
   spacing luce/ombra (px)      duty luce/ombra (0–1)     ← space_light/dark vs dash_light/dark
           │                           │
           │                    periodo tratto (px)       ← dash_period; non legato nascosto a space
           │                           │
           │                    irregolarità (opz.)       ← mod_jitter sul periodo/duty
           │                           │
           │                    eccezione pianura        ← dash_drop; cutoff = taglio netto
           └───────────┬───────────────┘
                       │
              carattere linea (mod_*)                   ← breaks, wobble, steep?, …
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
6. **Quantizzazione input (opzionale):** soglie / livelli discreti sul tono che
   alimenta il dash; es. due stati → zone scure a linea continua, zone chiare a
   tratteggio regolare con duty scelto (0.5, 0.2, 0.1, …) senza `mod_jitter`.
7. **Spacing in px agli estremi:** `space_light` / `space_dark` come distanze
   attese tra linee (non moltiplicatori su `space`); refactor congiunto a (6) e
   a `dash_period` per evitare doppi agganci nascosti.

## Riferimenti nel codice attuale

- Spezzatura e duty: `crates/burin-map-core/src/dashes.rs`
- Campo spacing per cella: `crates/burin-map-core/src/fields.rs`
- Definizioni e default: `crates/burin-map-core/src/style.rs` (macro `style!`)
- Acqua e anelli shore (usa `space` × mul): `crates/burin-map-core/src/water.rs`
- Discussione utente: conversazione di design, 2026

## Altro documentato altrove

- Ombreggiatura (`input_*`, `sun_*`) come sorgente del tono (condivisa da dash
  e spacing).
- Land-edge fray: [land-edge-fray.md](land-edge-fray.md).
