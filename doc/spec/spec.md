# Spécification du Projet : ttySeq

## 1. Vue d'ensemble

### 1.1 Description
Séquenceur musical hybride audio/MIDI/CV optimisé pour systèmes à faibles ressources (Raspberry Pi, anciens ordinateurs), destiné à la **performance live** de musique électronique. Le système se compose de deux parties : un environnement de préparation (webapp/plugin) et un runtime de performance (terminal).

> Le modèle de données détaillé (hiérarchie Project → Song → Section → Clip, comportements de fin de section, décisions actées et questions ouvertes) est défini dans [data-model.md](data-model.md).


## 2. Philosophie et Objectifs

### 2.1 Philosophie
- **Performance avant composition** : outil de scène, pas DAW
- **Légèreté** : fonctionner sur hardware minimal
- **Fiabilité** : latence minimale, stabilité maximale
- **Modularité** : support de multiples interfaces et sorties
- **Préparation/Exécution séparées** : composer confortablement, performer efficacement

### 2.2 Objectifs principaux
1. Lecture synchronisée de pistes audio et MIDI/CV
2. Latence audio < 10ms
3. Interface terminal (TUI) fournissant un affichage synchrone
4. Déclenchement et synchronisation via des messages standards (MIDI, DIN sync, Grid, 1-24 PPQ...) rendant possible l'utilisation de contrôleurs externes
5. Stabilité en conditions live
6. Transport simple : une seule commande **Play/Stop** (cf. [data-model.md §3](data-model.md#3-comportements-de-fin-de-section))

---

## 3. Architecture Système

### 3.1 Composants principaux

ttySeq adopte une architecture **client/serveur unifiée dans un binaire unique** : un engine séquenceur central, une couche protocole, et des clients qui s'y branchent (TUI, CLI, intégrations OSC externes). Le détail des modes de lancement et du protocole est défini en §3.3.

```
+-----------------------------------------------------+
|           ENVIRONNEMENT DE PRÉPARATION              |
|                                                     |
|  +-------------+          +--------------+          |
|  |   WebApp    |    OU    |   Plugin     |          |
|  |   (Tauri)   |          |   Ableton    |          |
|  +------+------+          +------+-------+          |
|         |                        |                  |
|         +------------+-----------+                  |
|                      |                              |
|              +-------v--------+                     |
|              |  Projet File   |                     |
|              |  (.toml/.yaml) |                     |
|              +-------+--------+                     |
+----------------------|------------------------------+
                       |
                       | Export/Transfer
                       |
+----------------------v------------------------------+
|              RUNTIME DE PERFORMANCE                 |
|                                                     |
|  +--------+  +--------+  +-----------------+        |
|  |  TUI   |  |  CLI   |  |   OSC clients   |        |
|  |        |  |        |  |  (SC, Tidal…)   |        |
|  +---+----+  +---+----+  +--------+--------+        |
|      |           |                |                 |
|      | MPSC      | Unix socket    | UDP loopback    |
|      | (in-proc) | (toujours)     | (opt-in)        |
|      |           |                |                 |
|      +-----+-----+----------------+                 |
|            |          (cf. §3.3)                    |
|     +------v-------+                                |
|     |   Couche     |                                |
|     |  protocole   |                                |
|     +------+-------+                                |
|            |                                        |
|     +------v-------+                                |
|     |    Engine    |                                |
|     |  séquenceur  |                                |
|     +------+-------+                                |
|            |                                        |
|   +--------+--------+                               |
|   |        |        |                               |
|   v        v        v                               |
| +-----+ +------+ +-----+                            |
| |Audio| | MIDI | | CV  |                            |
| | Out | | Out  | | Out |                            |
| +-----+ +------+ +-----+                            |
+-----------------------------------------------------+
```

### 3.2 Technologies envisagées

#### Runtime de performance (Core)
- **Langage** : Rust
- **Audio** : `cpal` (abstraction cross-platform : ALSA/PipeWire, CoreAudio)
- **MIDI** : `midir`
- **TUI** : `ratatui`
- **Configuration** : `serde` + TOML *(acté, cf. [data-model.md §5](data-model.md#5-décisions-actées-))*
- **Thread priorité** : `thread-priority`
- **IPC interne** : `std::sync::mpsc` ou `crossbeam-channel`, ring buffer lock-free `rtrb` pour le chemin audio
- **IPC externe** : socket Unix natif (`std::os::unix::net`), OSC via `rosc` *(à confirmer)*
- **Sérialisation wire** : `serde` + MessagePack (`rmp-serde`) ou JSON (`serde_json`) *(à trancher, cf. §3.3.4)*

#### Environnement de préparation
- **Option 1 - WebApp** : Tauri (Rust + HTML/CSS/JS)
- **Option 2 - Plugin Ableton** : Max4Live ou Python (extension future)

### 3.3 Architecture client/serveur et modes de lancement

ttySeq est structuré comme un **engine séquenceur** entouré d'une couche protocole, à laquelle se branchent un ou plusieurs **clients**. Le même vocabulaire de messages est utilisé en interne (entre threads, in-process) et en externe (entre processus, via socket ou OSC), avec une sérialisation appliquée uniquement quand un client distant est concerné.

Les clients se répartissent en trois familles : les **frontends humains** (TUI, CLI), les **surfaces de contrôle matérielles** (Norns Shield/Fates, Monome Grid, contrôleurs MIDI — cf. §3.3.7) et les **intégrations externes** (live coding, scripts — cf. §4.2.5).

Cette structure permet de garder un binaire unique pour les usages simples et solo, tout en ouvrant la voie à des frontends variés (CLI, web, contrôleurs externes, scripts) sans réécrire le cœur.

#### 3.3.1 Modes de lancement

Le binaire `ttyseq` peut être invoqué de quatre façons selon le contexte :

| Invocation | Mode | Comportement |
|---|---|---|
| `ttyseq` | Embedded | Engine + TUI dans le même process (cas par défaut, usage solo) |
| `ttyseq daemon` | Headless | Engine seul, sans TUI, expose l'IPC |
| `ttyseq attach` | Client TUI | TUI qui se connecte à un daemon existant |
| `ttyseq <subcommand>` | One-shot | Commande CLI qui envoie un message au daemon, attend la réponse, termine |

Quel que soit le mode, l'engine se comporte de la même façon et les clients communiquent avec lui via les mêmes types de messages.

#### 3.3.2 Protocole interne (dogfood léger)

Un seul vocabulaire de messages — `EngineCommand` (commandes envoyées à l'engine) et `EngineEvent` (événements émis par l'engine) — est utilisé indépendamment du transport.

- **Transport in-process** : les messages traversent une MPSC channel Rust, sans sérialisation. Le TUI embarqué utilise ce chemin.
- **Transport externe** : les mêmes messages sont sérialisés via `serde` au passage du socket Unix ou de l'interface OSC.

Cette approche garantit qu'aucune capacité du TUI ne soit indisponible aux clients externes, et inversement. Le surcoût d'écriture initial se limite à la discipline de garder les messages sous forme de données pures (pas de closures, pas de pointeurs partagés).

#### 3.3.3 Transports externes par défaut

Le comportement par défaut est asymétrique :

- **Socket Unix** : ouvert systématiquement, à l'emplacement `$XDG_RUNTIME_DIR/ttyseq.sock`. Pas de surface réseau, pas de port ouvert. Sert au CLI local et aux frontends riches (webapp future, etc.).
- **OSC sur UDP** : désactivé par défaut. S'active explicitement via configuration ou flag de lancement, et bind par défaut sur `127.0.0.1` (loopback only). Sert à l'intégration avec des environnements de live coding (SuperCollider, TidalCycles, etc.).

L'asymétrie reflète une asymétrie de risque et d'intention : le socket Unix local est essentiellement gratuit et toujours utile ; OSC engage un port réseau et a vocation à être activé délibérément.

#### 3.3.4 Format de sérialisation

À trancher en phase d'implémentation entre :

- **JSON** : lisible humainement, debug facile, supporté partout, mais verbeux.
- **MessagePack** : binaire compact, schéma identique à JSON côté Rust via `serde`, inspectable avec des outils dédiés.
- **Bincode** : binaire Rust-spécifique, très rapide, mais difficile à consommer depuis d'autres langages.

OSC a son propre format binaire imposé par sa spec. Pour le socket Unix, MessagePack apparaît comme un bon compromis (compacité + interopérabilité), à confirmer.

#### 3.3.5 Modèle requête/réponse et abonnement

L'engine peut être interrogé en **requête/réponse** (un client envoie une `EngineCommand` portant un `request_id`, attend l'`EngineEvent` correspondant), et peut aussi être **écouté en flux** par les clients qui veulent suivre l'état en temps réel (TUI, webapp, contrôleurs OSC). Le protocole prévoit donc une notion d'**abonnement** : un client peut demander à recevoir tous les événements d'une catégorie (transitions de section, lancement de clips, changements de transport, etc.).

#### 3.3.6 Sync tempo : sujet distinct

L'IPC décrite ici sert au **contrôle sémantique** : lancement de clips, transitions de sections, changements de transport, mutations d'état. Elle n'est pas conçue pour la synchronisation tempo à l'échantillon près. Les protocoles standards pour ce besoin (MIDI Clock, Ableton Link) sont à traiter séparément (cf. §4.6 et sujets différés en §10).

#### 3.3.7 Surfaces de contrôle in-process et politique de chargement

Les surfaces de contrôle matérielles (Shield/Fates, Grid, contrôleurs MIDI) sont des **adaptateurs** : elles traduisent les événements hardware en `EngineCommand`, et les `EngineEvent` en feedback (LEDs, écran). Ce sont des clients du protocole comme les autres.

**Modèle in-process.** Les surfaces tournent dans le binaire unique : chaque surface est un thread dédié, client du protocole via MPSC, spawné au démarrage selon la configuration. Aucune surface ne touche au thread audio. L'intérêt sur scène est d'avoir un seul process à superviser (un seul service systemd, un seul point de défaillance). Et comme les surfaces ne parlent que le protocole (données pures), l'extraction ultérieure d'une surface vers un process séparé (socket Unix) resterait possible sans réécriture : in-process vs out-of-process est un détail de déploiement, pas d'architecture.

Esquisse illustrative (non normative) :

```rust
trait Surface: Send {
    fn run(self: Box<Self>, cmds: Sender<EngineCommand>, events: EventSubscription);
}
```

**Politique de chargement.** Deux mécanismes complémentaires, et rien d'autre :

- **Compile-time (cargo features)** pour le code spécifique à une plateforme — ex. une feature `norns-shield` (framebuffer + GPIO Linux) présente dans le build Raspberry Pi, absente du build macOS de développement.
- **Runtime (configuration système)** pour l'activation : une section par surface (`[surface.shield]`, `[surface.grid]`, `[[surface.midi]]` avec fichier de mapping).

Pas de chargement dynamique (`dlopen`) : Rust n'a pas d'ABI stable, et cette fragilité est inacceptable pour un instrument de scène.

**Hotplug.** Les surfaces matérielles doivent tolérer la déconnexion/reconnexion : un contrôleur débranché puis rebranché en cours de live doit se réattacher sans intervention.

---

## 4. Fonctionnalités Détaillées

### 4.1 Gestion des pistes

#### Types de pistes

Une track est un **conduit de sortie**, défini une fois pour tout le projet. Elle ne contient pas de données musicales — le contenu vit dans les clips, à l'intérieur des sections de chaque song (cf. [data-model.md §2.2](data-model.md#22-track--un-conduit-de-sortie)).

1. **Pistes Audio Playback**
   - Format : WAV, FLAC (via `symphonia`)
   - Lecture temps réel avec buffer minimal
   - Contrôle volume/gain, état muet
   - Slot pour chaîne de plugins (voir §4.5)

2. **Pistes MIDI**
   - Fichiers `.mid` externes (MIDI standard), pas de séquences inlinées
   - Support clock MIDI externe *(à valider)*

3. **Pistes CV** (Control Voltage) — placeholder architectural

   Trois étapes prévues, dont seule la première est dans le MVP :

   - **MVP** : pas de type "CV" explicite. L'envoi de CV passe par une **piste MIDI** routée vers un convertisseur MIDI-to-CV externe (séquenceur hardware, module de conversion). C'est suffisant pour les notes, gates et triggers.
   - **Post-MVP — CV brut via audio multicanal** : une piste audio peut être routée vers des canaux physiques dédiés à la modulation. Le signal est traité comme de l'audio par ttySeq, mais doit être bypassé strictement en aval (cf. §4.4).
   - **Post-MVP — MIDI-to-CV natif** : type de piste générant des signaux DC calibrés (V/Oct, gates) sur des canaux audio dédiés, à la manière du CV Tools d'Ableton. Permet d'éditer en MIDI tout en sortant du CV propre, sans hardware de conversion.

#### Organisation

Le projet suit une hiérarchie en 4 niveaux : **Project → Song → Section → Clip**, avec les tracks comme conduits transverses (cf. [data-model.md §2](data-model.md#2-hiérarchie)).

- Chaque **song** a son propre tempo (BPM) et sa signature rythmique
- Chaque **section** a une durée fixe (en mesures/beats) et un comportement de fin (`Advance`, `Stop`, `LoopFull`, `LoopTail`)
- Chaque **clip** est un pointeur vers un fichier externe (audio ou MIDI), joué sur une track dans une section

#### Routing audio

Le routage est **direct** : chaque piste audio est routée vers la sortie principale ou vers un canal physique spécifique. Pas de bus auxiliaires, pas de sends, pas de matrix de routing.

Le **bus de sommation principal** est interne à ttySeq : c'est le seul point où plusieurs pistes audio sont mixées entre elles, à destination de la sortie stéréo principale. Pour des sorties multicanaux dédiées (par track ou groupe de tracks), le routage reste direct vers le canal physique cible.

```toml
[[tracks]]
id = 1
name = "Kick"
type = "audio"
output = { type = "interface_channel", channel = 1 }
volume = 0.85

[[tracks]]
id = 2
name = "Pad"
type = "audio"
output = { type = "interface_channel", channel = 3 }
volume = 0.60
```

Le contenu musical (fichiers audio) est référencé dans les **clips** au niveau des sections, pas dans la définition des tracks (cf. [data-model.md §2.5](data-model.md#25-clip--le-contenu-musical-à-jouer)).

### 4.2 Interfaces de contrôle

Les interfaces décrites ci-dessous (TUI, Monome Grid, contrôleurs MIDI, surface Shield/Fates) sont toutes des **clients du protocole §3.3**. Elles ne dialoguent pas directement avec le routeur audio/MIDI : elles envoient des `EngineCommand` et reçoivent des `EngineEvent` via le transport approprié (MPSC in-process pour le TUI embarqué et les surfaces de contrôle, socket Unix ou OSC pour les clients externes). Les surfaces matérielles sont des adaptateurs in-process au sens de §3.3.7. Cette uniformité permet d'ajouter de nouvelles interfaces sans modifier l'engine.

#### 4.2.1 Terminal (TUI)
```
+-----------------------------------------------------+
| ttySeq v1.0    Song: Opener    Section: Intro   128 BPM  > |
+------------------------------------------------------------+
| Track 1  [########..] Drums (audio)      Vol: 85%          |
| Track 2  [##########] Bass (midi)        Mute              |
| Track 3  [####......] Pad (audio)        Vol: 60%          |
| Track 4  [..........] Lead (cv)          Armed             |
+------------------------------------------------------------+
| [1] Intro  [2] Couplet  [3] Refrain*  [4] Outro           |
+------------------------------------------------------------+
| > Play/Stop                                                |
+-----------------------------------------------------+
```

Modes d'interface :
- **Mode Performance** : vue simplifiée, gros visuels
- **Mode Tracker** : vue grille type Renoise/FastTracker
- **Mode Mixer** : contrôle des niveaux et routing

#### 4.2.2 Monome Grid
- Déclenchement de scènes/clips
- Mute/Solo de pistes
- Patterns step sequencer
- Feedback LED synchronisé
- Dialogue via le démon `serialosc` : l'adaptateur grid est in-process côté ttySeq et communique avec serialosc en OSC

#### 4.2.3 MIDI Controller
- Mapping libre via fichier de configuration
- Support CC/Notes/Program Change

#### 4.2.4 Surface Norns Shield / Fates

Écran OLED, encodeurs et boutons du hardware Norns Shield/Fates, utilisés comme **panneau avant alternatif au TUI** : affichage synchrone de l'état (song, section, transport) et contrôles directs. Architecturalement, c'est une surface de contrôle comme les autres (cf. §3.3.7), avec un feedback plus riche.

- Compilée derrière la feature cargo `norns-shield` (framebuffer + GPIO, Linux/Raspberry Pi uniquement)
- Acquis du spike : le pilotage de l'écran via les sources de Norns est validé ; boutons et encodeurs restent à tester

#### 4.2.5 Instrument externe live coding

Au-delà du contrôle de session (accessible à tout client OSC ou socket, cf. §3.3), un environnement de live coding (Haskell, SuperCollider…) peut fonctionner comme **instrument parallèle synchronisé** :

- ttySeq est **maître de clock** : le process externe reçoit clock et transport (MIDI Clock, cf. §4.6 ; Ableton Link ou ticks OSC timestampés à terme, cf. §10)
- Le process externe **sort sa musique lui-même** : MIDI directement vers le hardware, ou audio dans le graphe JACK vers le démon mixer (§4.4) — jamais via une entrée audio de ttySeq (§4.4.1)

Une intégration plus profonde — track « live » dont le contenu est un flux d'événements musicaux timestampés routé par l'engine — est un sujet différé (cf. §10).


### 4.3 Sorties et Routing

#### Configuration modulaire

La configuration est séparée en deux fichiers distincts : la configuration du programme (audio backend, sorties physiques) et le fichier de projet (pistes, BPM, etc.).

**Configuration système (`config.toml`) :**
```toml
[audio]
sample_rate = 48000
buffer_size = 512   # 512-1024 recommandé sur Raspberry Pi
backend = "alsa"    # ou "jack", "coreaudio"

[[outputs]]
id = "main"
type = "audio"
device = "hw:0,0"
channels = [0, 1]

[[outputs]]
id = "aux"
type = "audio"
device = "hw:0,0"
channels = [2, 3]

[[outputs]]
id = "drums_midi"
type = "midi"
port = "TR-8S"

[[outputs]]
id = "modular_cv"
type = "cv"
device = "crow"
channels = [1, 2, 3, 4]
```

**Fichier de projet (`project.toml`) :**
```toml
[metadata]
name = "My Live Set"
author = "Max"

# Tracks = conduits de sortie (fixes pour tout le live)
[[tracks]]
id = 1
name = "Kick"
type = "audio"
output = { type = "interface_channel", channel = 1 }
volume = 0.85

[[tracks]]
id = 2
name = "Bass"
type = "midi"
output = { type = "device", name = "TR-8S" }
channel = 1

# Tempo et signature par song
[[songs]]
id = 1
name = "Opener"
tempo = 128.0
time_signature = [4, 4]

[[songs.sections]]
name = "Intro"
length = { bars = 8 }
on_end = "advance"

[songs.sections.clips]
1 = { type = "audio", file = "audio/kick_intro.wav" }
2 = { type = "midi", file = "midi/bass.mid", playback = "loop" }

setlist = [1]
```

#### Types de sorties supportées
1. **Audio** : ALSA/PipeWire, JACK, CoreAudio
2. **MIDI** : Ports MIDI standard
3. **CV** : Monome Crow via USB série

### 4.4 Intégration audio en chaîne live

Beaucoup de performeurs utilisent leur DAW non seulement pour le séquençage mais aussi comme **mixer de fin de chaîne** : sommation de plusieurs sources (séquenceur, modulaire, autres machines) et traitement master (compression, limitation). Avec ttySeq comme remplaçant du DAW, cette fonction n'est volontairement pas intégrée — au nom de la simplicité et de la séparation des responsabilités.

#### 4.4.1 Périmètre de ttySeq

ttySeq se limite strictement au séquençage et à la sortie audio :

- Les pistes audio internes sont sommées vers une **sortie stéréo principale** (cf. §4.3)
- Aucune entrée audio externe n'est traitée par ttySeq
- Aucun effet master n'est appliqué

#### 4.4.2 Démon mixer externe (recommandation, hors MVP)

Pour les performeurs qui ont besoin d'un mix master (sommation de plusieurs sources + compression/limitation), la solution recommandée est un **process séparé** sur la même machine, branché en aval de ttySeq via une couche audio inter-process (JACK, ou PipeWire en mode JACK-compatible).

Ce démon — appelons-le `ttyseq-mixer` — n'est pas développé dans la phase MVP, mais sa place dans la chaîne est documentée dès maintenant pour cadrer l'architecture. Il pourrait être :

- Un programme dédié écrit dans ce projet (post-MVP)
- Un outil tiers existant (`mod-host`, `Carla`, etc.) configuré pour ce rôle
- Absent — remplacé par du hardware (table de mix + compresseur master)

**Topologie cible :**

```
ttySeq ───────────┐
modulaire ────────┤
autres sources ───┤
                  ├─► démon mixer ─► sortie physique
                  │   (sommation +
                  │    master FX)
```

#### 4.4.3 Routing audio musical / CV distinct

Si le démon mixer route aussi des canaux CV (cf. §4.1 — CV brut via audio multicanal), il doit distinguer **deux chemins de signal** :

- **Audio musical** (ttySeq L/R, sources externes) → sommation → effets master (compresseur, limiteur, EQ) → sortie principale
- **CV** (canaux audio dédiés à la modulation) → bypass strict, aucun traitement → sorties physiques correspondantes

Cette distinction est critique : un compresseur ou un limiteur appliqué à un signal CV altèrerait les enveloppes et les tensions de pitch.

#### 4.4.4 Latence inter-process

Avec JACK ou PipeWire, deux processus dans le même graphe audio partagent le même cycle de traitement (mémoire partagée, period commune). La latence inter-process est essentiellement nulle, contrairement à un chaînage via deux drivers ALSA séparés.

#### 4.4.5 Configuration des sorties

Côté `config.toml`, la sortie principale de ttySeq peut être routée vers un port JACK plutôt que vers un canal hardware direct, pour permettre l'insertion d'un démon mixer en aval :

```toml
[audio]
backend = "jack"

[[outputs]]
id = "main"
type = "audio"
ports = ["mixer:in_L", "mixer:in_R"]
```

En l'absence de démon mixer, la sortie est routée directement vers les canaux physiques.

### 4.5 Système de Plugins (architecture prévue, implémentation reportée)

L'architecture plugin est définie dès maintenant pour éviter un refactor futur, mais les implémentations concrètes arrivent en phase 3. En phase 1-2, les pistes passent leur signal directement sans traitement.

#### 4.4.1 Architecture

Chaque piste expose un slot `Vec<Box<dyn StagePlugin>>`. À l'initialisation, ce vecteur est vide — le signal traverse sans traitement. Les plugins s'y branchent plus tard sans modifier l'architecture.

```rust
pub trait StagePlugin: Send + Sync {
    fn info(&self) -> PluginInfo;
    fn init(&mut self, sample_rate: f32);
    fn process(&mut self, input: &AudioBuffer, output: &mut AudioBuffer, ctx: &ProcessContext);
    fn set_parameter(&mut self, id: &str, value: f32);
    fn get_parameter(&self, id: &str) -> f32;
    fn parameters(&self) -> Vec<ParameterDef>;
}

pub struct ProcessContext {
    pub sample_rate: f32,
    pub buffer_size: usize,
    pub tempo: f32,
    pub time_position: f64,
}
```

Le champ `plugins` est déjà présent dans le format de configuration des pistes, mais ignoré en l'absence d'implémentation :

```toml
[[tracks]]
id = 1
name = "Kick"
type = "audio"
output = { type = "interface_channel", channel = 1 }
plugins = []
```


### 4.6 Synchronisation

- **Master Clock interne** : tempo défini par song (cf. [data-model.md §2.3](data-model.md#23-song--une-chanson-du-setlist))
- **MIDI Clock** : In/Out, master/slave — la sortie clock sert aussi à synchroniser des instruments externes type live coding (cf. §4.2.5)
- **Ableton Link** *(sujet différé, hors MVP)* : intégration envisagée à terme pour la synchronisation tempo inter-applications avec des environnements de live coding (cf. §3.3.6 et §10)

---

## 5. Format de Projet

### 5.1 Structure de fichier
```
mon_set_live/
├── project.toml          # Configuration principale (TOML, acté)
├── audio/                # Fichiers audio (WAV/FLAC)
│   ├── drums/
│   │   ├── intro.wav
│   │   └── drop.wav
│   └── pad.flac
├── midi/                 # Séquences MIDI (.mid)
│   ├── bass.mid
│   └── lead.mid
├── cv/                   # Courbes CV (hors MVP)
│   └── filter_env.cv
└── mappings/             # Contrôleurs (hors MVP)
    ├── grid.toml
    └── midi_controller.toml
```

Les sections sont définies **inline** dans le fichier projet, à l'intérieur de chaque song — pas dans des fichiers séparés (cf. [data-model.md §2.4](data-model.md#24-section--lunité-fondamentale-du-live)).

### 5.2 Format de fichier projet (TOML)
```toml
[metadata]
type = "ttyseq-project"
version = "1.0"
created = "2026-01-26"
modified = "2026-04-02"
author = "Artist Name"

[project]
name = "Live Set Winter 2026"
master_volume = 1.0

[sync]
mode = "internal"  # internal, midi_clock
```

Le tempo et la signature rythmique sont définis **par song**, pas au niveau du projet (cf. [data-model.md §2.3](data-model.md#23-song--une-chanson-du-setlist)).

---

## 6. Priorités de Développement

### Phase 1 : MVP
- [ ] Engine de playback audio basique
- [ ] Séquenceur MIDI simple
- [ ] Interface TUI minimale
- [ ] Lecture de fichiers de projet (TOML)
- [ ] Sortie audio ALSA/JACK
- [ ] Sortie MIDI basique
- [ ] Routing direct piste → sortie physique

### Phase 2 : Performance Core
- [ ] Synchronisation précise audio/MIDI
- [ ] Optimisation latence
- [ ] Gestion de scènes/sections
- [ ] Protocole interne (cf. §3.3) : types `EngineCommand` / `EngineEvent`, transport MPSC in-process, transport socket Unix
- [ ] Modes de lancement : `ttyseq`, `ttyseq daemon`, `ttyseq attach`, `ttyseq <subcommand>`
- [ ] Interface OSC opt-in pour intégration live coding (loopback par défaut)
- [ ] Contrôle clavier étendu
- [ ] Tests sur Raspberry Pi

### Phase 3 : Contrôleurs externes
- [ ] Support Monome Grid
- [ ] MIDI mapping configurable
- [ ] Feedback visuel (LED)
- [ ] Surface Norns Shield/Fates (écran, encodeurs, boutons — feature `norns-shield`)

### Phase 4 : Sorties multicanaux et CV brut
- [ ] Multi-sorties audio (canaux multiples par carte son)
- [ ] Pistes audio routées vers canaux CV dédiés (modulation, gates)

### Phase 5 : Démon mixer (optionnel)
- [ ] `ttyseq-mixer` séparé : sommation multi-sources + master FX
- [ ] Routing audio musical / CV distinct (bypass strict pour le CV)

### Phase 6 : MIDI-to-CV natif
- [ ] Type de piste "MIDI-to-CV" : génération de signaux DC calibrés (V/Oct, gates)
- [ ] Calibration par sortie

### Phase 7 : Environnement de préparation
- [ ] WebApp Tauri basique
- [ ] Éditeur de projet visuel
- [ ] Timeline drag & drop
- [ ] Export vers runtime

### Phase 8 : Écosystème & Polish
- [ ] Plugin Ableton (optionnel)
- [ ] Documentation complète
- [ ] Presets et templates

---

## 7. Spécifications Techniques

### 7.1 Performances cibles
- **Latence audio** : < 10ms (round-trip)
- **Jitter MIDI** : < 1ms
- **CPU usage** : < 30% sur Raspberry Pi 4
- **RAM** : < 512MB pour projet moyen
- **Démarrage** : < 3 secondes

### 7.2 Compatibilité
- **OS** : Linux (priorité), macOS, Windows
- **Hardware minimal** :
  - Raspberry Pi 3B+ ou supérieur
  - 1GB RAM minimum
  - Carte son USB (améliore la latence)
- **Hardware recommandé** :
  - Raspberry Pi 4 (4GB) avec kernel `PREEMPT_RT`
  - Carte son USB class-compliant
  - Interface MIDI USB

### 7.3 Limites techniques
- **Pistes audio simultanées** : 16 minimum, 32 recommandé
- **Pistes MIDI simultanées** : 64
- **Sorties CV** : limité par la carte son (CV brut transitant par des canaux audio dédiés, post-MVP)
- **Taille fichier audio** : Illimitée (streaming)
- **Résolution audio** : 16/24-bit, 44.1–96kHz

---

## 8. Expérience Utilisateur

### 8.1 Workflow typique

#### Préparation (sur ordinateur)
1. Créer nouveau projet dans WebApp
2. Importer fichiers audio/MIDI
3. Organiser en songs/sections/clips
4. Configurer routing des sorties
5. Mapper contrôleurs
6. Exporter projet

#### Performance (sur Raspberry Pi/scène)
1. Copier projet sur Pi (USB/réseau)
2. Lancer : `ttyseq run mon_set_live/`
3. Interface TUI s'affiche
4. Connecter contrôleurs (Grid, MIDI)
5. Performer : Play/Stop, mute/solo, contrôle paramètres
6. Logs sauvegardés pour post-mortem

### 8.2 Cas d'usage principaux

#### 1. DJ/Producer — Set live hybride
**Setup :**
- Backtracks audio (stems : drums, bass, pads)
- Synthé hardware connecté en MIDI
- Grid Monome pour déclenchement de scènes

```toml
# Tracks = conduits de sortie (fixes pour tout le live)
[[tracks]]
id = 1
name = "Drums"
type = "audio"
output = { type = "interface_channel", channel = 1 }

[[tracks]]
id = 2
name = "Bass"
type = "audio"
output = { type = "interface_channel", channel = 2 }

[[tracks]]
id = 3
name = "Synth Hardware"
type = "midi"
output = { type = "device", name = "Synth" }
channel = 1

# Le contenu (fichiers audio/MIDI) est dans les clips des sections
[[songs]]
id = 1
name = "Opener"
tempo = 128.0
time_signature = [4, 4]

[[songs.sections]]
name = "Drop"
length = { bars = 16 }
on_end = "advance"

[songs.sections.clips]
1 = { type = "audio", file = "audio/drums.wav", playback = "loop" }
2 = { type = "audio", file = "audio/bass.wav", playback = "loop" }
3 = { type = "midi", file = "midi/lead.mid", playback = "loop" }
```

#### 2. Live band électronique — Pistes + click
**Setup :**
- Pistes audio : backing vocals, synthés enregistrés
- Click track vers sortie dédiée (casque batteur)
- Sample player MIDI pour FX live
- MIDI vers synthé hardware

```toml
# Tracks = conduits de sortie
[[tracks]]
id = 1
name = "Click Track"
type = "audio"
output = { type = "interface_channel", channel = 3 }  # Casque musiciens

[[tracks]]
id = 2
name = "Backing Vocals"
type = "audio"
output = { type = "interface_channel", channel = 1 }

# Clips dans les sections
[[songs]]
id = 1
name = "Set complet"
tempo = 120.0
time_signature = [4, 4]

[[songs.sections]]
name = "Couplet 1"
length = { bars = 16 }
on_end = "advance"

[songs.sections.clips]
1 = { type = "audio", file = "audio/click.wav", playback = "loop" }
2 = { type = "audio", file = "audio/bvox_v1.wav" }
```

#### 3. Prototype Eurorack/Hardware
**Setup :**
- CV output vers modules (via Crow)
- MIDI vers séquenceurs hardware
- Séquences audio de drones/textures en playback

```toml
# Tracks = conduits de sortie
[[tracks]]
id = 1
name = "CV Envelope"
type = "cv"
output = { type = "device", name = "crow" }
channel = 1

[[tracks]]
id = 2
name = "Drone"
type = "audio"
output = { type = "interface_channel", channel = 1 }

# Clips dans les sections
[[songs]]
id = 1
name = "Ambient Set"
tempo = 80.0
time_signature = [4, 4]

[[songs.sections]]
name = "Texture"
length = { bars = 32 }
on_end = "loop_full"
clips = { 2 = { type = "audio", file = "audio/drone.wav", playback = "loop" } }
```

---

## 9. Différenciation et Concurrence

| Critère | ttySeq | Ableton Live | Norns | Orca | Bitwig |
|---------|--------|--------------|-------|------|--------|
| Ressources | Très faible | Élevées | Moyen | Faible | Élevées |
| Latence | < 10ms | 10–20ms | < 10ms | Variable | 10–20ms |
| Audio+MIDI (+CV prévu) | ✅ | ✅ | ✅ | ❌ (MIDI only) | ✅ |
| Interface terminal | ✅ | ❌ | ❌ | ✅ | ❌ |
| Prix | Gratuit | €349+ | €650 | Gratuit | €399 |
| Orienté live | ✅ | ✅ | ✅ | ⚠️ | ✅ |
| Modularité hardware | ✅ | ⚠️ | ✅ | ⚠️ | ⚠️ |

### Positionnement unique
- **Le seul** séquenceur terminal complet audio+MIDI+CV
- **Le plus léger** pour performance live professionnelle
- **Open source** et hackable
- **Conçu pour Raspberry Pi** dès le départ

---

## 10. Risques et Mitigation

| Risque | Impact | Probabilité | Mitigation |
|--------|--------|-------------|------------|
| Latence audio trop élevée | Haut | Moyen | Kernel PREEMPT_RT, JACK, buffers optimisés |
| Instabilité en live | Critique | Faible | Tests extensifs, mode safe, logs détaillés |
| Complexité TUI | Moyen | Élevé | Itération UX, modes simplifiés |
| Compatibilité CV hardware | Moyen | Moyen | Support limité à devices populaires (Crow) |
| Scope creep | Élevé | Élevé | MVP strict, roadmap claire |
| Latence IPC (socket Unix, OSC) | Faible | Faible | Le thread audio RT ne touche jamais à l'IPC ; transports loopback uniquement par défaut |

### Sujets différés (post-MVP)

Topics architecturalement reconnus mais hors scope du MVP et de ses extensions immédiates, listés ici pour mémoire :

- **Ableton Link** : synchronisation tempo inter-applications avec environnements de live coding (cf. §3.3.6, §4.6). Le besoin est réel pour l'intégration SuperCollider / TidalCycles / Sonic Pi en performance hybride, mais l'implémentation est différée pour ne pas alourdir le scope initial.
- **Sync tempo à l'échantillon près** : au-delà de MIDI Clock et Ableton Link, des mécanismes plus fins (synchro via timestamps OSC, intégration JACK transport) pourront être étudiés selon les besoins de performance.
- **Track live (instrument externe routé)** : track dont le contenu n'est pas un fichier mais un flux d'événements musicaux timestampés envoyé par un process externe (live coding, cf. §4.2.5) via OSC, ordonnancé par l'engine sur sa clock et joué sur la sortie de la track. L'instrument externe deviendrait une vraie track ttySeq (TUI, mute, sections). Nécessite une extension du protocole (événements musicaux timestampés, gestion du jitter par envoi anticipé des bundles, à la TidalCycles) et du modèle de données.

---


## 11. Licence et Open Source

### Licence recommandée
**GPL v3** ou **MIT** selon philosophie :
- GPL : garantit que les dérivés restent open source
- MIT : plus permissive, adoption plus large

### Dépendances principales
| Crate | Licence |
|-------|---------|
| `cpal` | Apache 2.0 |
| `midir` | MIT |
| `ratatui` | MIT |
| `serde` | MIT/Apache |
| `thread-priority` | MIT |
| `symphonia` | MPL 2.0 |

---

## 12. Communauté et Contribution

- **GitHub** : Code, issues, discussions
- **Discord/Forum** : Entraide, partage de sets
- **Documentation** : Wiki, tutoriels

---

## 13. Conclusion

**ttySeq** est positionné pour combler un vide dans l'écosystème : un séquenceur musical complet, léger, fiable, orienté performance live, fonctionnant sur hardware minimal. Son architecture volontairement simple (pas de bus internes, pas de sends, routing direct) garantit stabilité et maintenabilité.

**Next steps :**
1. Implémenter le modèle de données et la machine à états de session (tests de scénario, sans audio)
2. Prototyper le playback audio basique en Rust
3. Tester la latence sur Raspberry Pi 4
4. Développer le MVP TUI

---

**Document version** : 1.6
**Date** : 18 août 2026
**Auteur** : Spécification collaborative
