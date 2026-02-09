# Spécification du Projet : Séquenceur Musical Orienté Performance

## 1. Vue d'ensemble

### 1.1 Description
Séquenceur musical hybride audio/MIDI/CV optimisé pour systèmes à faibles ressources (Raspberry Pi, anciens ordinateurs), destiné à la **performance live** de musique électronique. Le système se compose de deux parties : un environnement de préparation (webapp/plugin) et un runtime de performance (terminal).

### 1.2 Propositions de noms

#### Orientés performance/live
- **Stage** / **StageSeq** ⭐ (recommandé)
- **Deck**
- **Set**
- **Cue**
- **Rig**
- **Backbone**
- **Anchor**
- **Bedrock**

#### Orientés modularité/technique
- **Nexus**
- **Flux**
- **Core** / **CoreSeq**
- **Hybrid** / **HybridSeq**
- **Lattice**
- **Weaver**

#### Orientés minimalisme
- **Sparse**
- **Lean**
- **Etch**
- **Bare**

#### Orientés terminal/technique
- **ttySeq** - Jeu de mots sur TTY (terminal) + Sequencer, très Unix-style

**Recommandation principale : STAGE** - évocateur, simple, mémorable, parfait pour la performance live.

---

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
4. Déclenchement et synchronisation via des messages standards (MIDI, DIN sync, OSC, Grid, 1-24 PPQ...) rendant possible l'utilisation de contrôleurs externes
5. Gestion flexible du routing de sorties MIDI, OSC, Grid, Audio...
6. Stabilité en conditions live

---

## 3. Architecture Système

### 3.1 Composants principaux

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
|              |  (.json/.yaml) |                     |
|              +-------+--------+                     |
+----------------------|------------------------------+
                       |
                       | Export/Transfer
                       |
+----------------------v------------------------------+
|           RUNTIME DE PERFORMANCE                    |
|                                                     |
|  +---------------------------------------+          |
|  |    Terminal Sequencer (Rust)         |          |
|  |                                       |          |
|  |  +----------+  +----------+           |          |
|  |  | Playback |  | Control  |           |          |
|  |  |  Engine  |  | Interface|           |          |
|  |  +----+-----+  +----+-----+           |          |
|  |       |             |                 |          |
|  |  +----v-------------v-----+           |          |
|  |  |   Audio/MIDI Router    |           |          |
|  |  +----+--------+------+---+           |          |
|  +-------|--------|------|---------------+          |
|          |        |      |                          |
|     +----v----+ +-v----+ +v----+                    |
|     | Audio  | | MIDI | | CV  |                     |
|     |  Out   | | Out  | | Out |                     |
|     +--------+ +------+ +-----+                     |
|                                                     |
|  +---------------------------------------+          |
|  |    Input Controllers                 |          |
|  |  • Terminal (Tracker/TUI)            |          |
|  |  • MIDI Controllers                  |          |
|  |  • Monome Grid                       |          |
|  |  • OSC Devices                       |          |
|  +---------------------------------------+          |
+-----------------------------------------------------+
```

### 3.2 Technologies envisagées

#### Runtime de performance (Core)
- **Langage** : Rust
- **Audio** : CPAL ou JACK
- **MIDI** : midir
- **TUI** : ratatui ou cursive
- **Configuration** : serde + TOML/JSON
- **Timing** : tokio pour l'async

#### Environnement de préparation
- **Option 1 - WebApp** : Tauri (Rust + HTML/CSS/JS)
- **Option 2 - Plugin Ableton** : Max4Live ou Python (extension future)

---

## 4. Fonctionnalités Détaillées

### 4.1 Gestion des pistes

#### Types de pistes
1. **Pistes Audio Playback**
   - Format : WAV, FLAC, MP3 (via libsndfile ou symphonia)
   - Lecture temps réel avec buffer minimal
   - Support fade in/out
   - Contrôle volume/pan
   - Chaîne de plugins insert

2. **Pistes Audio Input** ⭐ (nouveau)
   - Capture audio en temps réel depuis interface
   - Routage flexible vers n'importe quelle sortie ou piste
   - Monitoring avec latence compensation
   - Chaîne d'effets en temps réel
   - **Use cases :**
     * Guitare/voix avec effets → master mix
     * Synthé hardware → processing → resample
     * Microphone → vocoder → side-chain compression
     * Entrée stéréo → split → sorties multiples
     * Loopback interne pour re-routing

3. **Pistes Sample Player** ⭐ (nouveau)
   - Déclenchement de samples one-shot ou looped
   - Contrôle pitch/speed indépendants
   - Enveloppes ADSR
   - Multi-samples avec velocity layers
   - Slicing automatique (détection de transients)
   - **Use cases :**
     * Drum machine (kick, snare, hats)
     * One-shots FX déclenchés en live
     * Textures granulaires
     * Pad multi-échantillonné

4. **Pistes MIDI**
   - Séquences de notes/CC
   - Support clock MIDI externe
   - Quantization optionnelle
   - Transposition temps réel
   - Arpeggiator intégré

5. **Pistes CV** (Control Voltage)
   - Via interface externe (Monome Crow, Expert Sleepers, etc.)
   - Courbes de modulation
   - Gates/triggers
   - LFO intégrés


#### Organisation 
- Scènes/clips (inspiration Ableton) (à retravailler : plutôt que de s'inspirer d'Ableton comme le suggère Claude, on pourrait penser à une autre structuration mieux pensée pour le live)
- Groupes de pistes
- Mutliplication temporelle (x2, /2, etc.)
- Boucles par piste ou globales

#### Routing Audio Flexible ⭐ (nouveau)

Le système de routing matriciel permet de connecter n'importe quelle source à n'importe quelle destination :

**Sources disponibles :**
- Pistes audio playback
- Pistes audio input (entrées physiques)
- Sorties de pistes (post-fader)
- Sends auxiliaires
- Bus internes

**Destinations disponibles :**
- Sorties physiques (master, aux, casque...)
- Autres pistes (pour re-sampling ou traitement)
- Bus d'effets
- Enregistrement interne

**Exemples de routage :**
```toml
# Guitare → EQ → Reverb → Master + Enregistrement
[[tracks]]
id = 5
name = "Guitar In"
type = "audio_input"
input_device = "hw:1,0"  # Interface audio externe
input_channel = 0
plugins = ["eq3band", "reverb_hall"]
output = ["main", "recorder_track_1"]

# Synthé hardware → Filtre → Split vers 2 sorties différentes
[[tracks]]
id = 6
name = "Modular In"
type = "audio_input"
input_device = "hw:1,0"
input_channel = 1
plugins = ["lowpass_filter"]
output = ["main", "sidechain_bus"]

# Sample player déclenché par MIDI
[[tracks]]
id = 7
name = "Drum Sampler"
type = "sample_player"
samples_dir = "samples/drums/"
midi_input = "Launch Control"
output = "main"
```

**Cas d'usage avancés :**
- **Re-sampling en live** : Router une piste vers une autre piste en enregistrement
- **Side-chaining** : Envoyer un signal vers un bus de compression
- **Parallel processing** : Splitter un signal vers plusieurs chaînes d'effets
- **Multi-output** : Envoyer drums vers master + sortie dédiée pour ingé son

### 4.2 Interfaces de contrôle

#### 4.2.1 Terminal (TUI)
```
+-----------------------------------------------------+
| STAGE v1.0          Scene: Intro     BPM: 128  >   |
+-----------------------------------------------------+
| Track 1  [########..] Kick.wav         Vol: 85%    |
| Track 2  [##########] Bass_MIDI        Mute        |
| Track 3  [####......] Pad.wav          Vol: 60%    |
| Track 4  [..........] Lead_CV          Armed       |
+-----------------------------------------------------+
| [1] Intro  [2] Verse  [3] Chorus*  [4] Bridge     |
+-----------------------------------------------------+
| > Play/Pause  Space: Cue  Tab: Next  S: Stop      |
+-----------------------------------------------------+
```

Modes d'interface :
- **Mode Performance** : vue simplifiée, gros visuels
- **Mode Tracker** : vue grille type Renoise/FastTracker
- **Mode Mixer** : contrôle des niveaux et routage

#### 4.2.2 Monome Grid
- Déclenchement de scènes/clips
- Mute/Solo de pistes
- Patterns step sequencer
- Feedback LED synchronisé

#### 4.2.3 MIDI Controller
- Mapping libre via fichier de configuration
- Learn mode
- Support CC/Notes/Program Change

#### 4.2.4 OSC
- Protocole ouvert pour contrôle externe
- TouchOSC, Lemur, applications custom
- Bidirectionnel (feedback)

### 4.3 Sorties et Routing

#### Configuration modulaire
```toml
# Exemple de configuration

[project]
name = "My Live Set"
bpm = 128
time_signature = "4/4"

[audio]
sample_rate = 48000
buffer_size = 128
backend = "jack"  # ou "alsa", "coreaudio"

[[outputs]]
id = "main"
type = "audio"
device = "hw:0,0"
channels = [0, 1]

[[outputs]]
id = "sidechain"
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

[[tracks]]
id = 1
name = "Kick"
type = "audio"
file = "audio/kick.wav"
output = "main"
volume = 0.85

[[tracks]]
id = 2
name = "Bass"
type = "midi"
file = "midi/bass.mid"
output = "drums_midi"
channel = 1
```

#### Types de sorties supportées
1. **Audio** : ALSA, JACK, CoreAudio, WASAPI
2. **MIDI** : Ports MIDI standard
3. **CV** : Via interfaces USB (Crow, Expert Sleepers, FH-2)
4. **Réseau** : OSC out, MIDI over network

### 4.4 Système de Plugins ⭐ (nouveau)

Stage intègre un système de plugins léger et performant, orienté live performance. L'architecture privilégie la simplicité et la stabilité.

#### 4.4.1 Architecture des plugins

**Format natif : Stage Plugins (.sp)**
- Plugins écrits en Rust, compilés comme bibliothèques dynamiques
- API simple et documentée pour développeurs tiers
- Zero-copy audio processing
- Thread-safe par défaut
- Latence garantie < 1ms

**Compatibilité future (roadmap) :**
- CLAP (CLever Audio Plugin) - format moderne et ouvert
- LV2 (Linux seulement) - pour intégration avec écosystème existant

#### 4.4.2 Plugins essentiels inclus (built-in)

**Traitement dynamique :**
```toml
[[plugins]]
id = "compressor"
name = "Stage Compressor"
parameters = [
  { name = "threshold", min = -60.0, max = 0.0, default = -20.0, unit = "dB" },
  { name = "ratio", min = 1.0, max = 20.0, default = 4.0 },
  { name = "attack", min = 0.1, max = 100.0, default = 10.0, unit = "ms" },
  { name = "release", min = 10.0, max = 1000.0, default = 100.0, unit = "ms" },
  { name = "makeup", min = 0.0, max = 24.0, default = 0.0, unit = "dB" }
]
```

**Égalisation :**
- **EQ 3 bandes** : Bass / Mid / Treble (style DJ mixer)
- **EQ paramétrique** : 4 bandes avec Q, gain, fréquence
- **Filtre passe-haut/bas** : Pour nettoyage

**Effets temporels :**
- **Reverb** : Algorithme simple type Freeverb
  - Room, Hall, Plate
  - Taille, damping, dry/wet
- **Delay** : Ping-pong, tempo-sync
- **Chorus** : Épaississement

**Utilitaires :**
- **Gain** : Volume simple avec visualisation
- **Pan** : Panoramique stéréo
- **Limiter** : Protection sortie (toujours actif sur master)
- **Gate** : Réduction bruit
- **Analyse** : Spectrogramme, oscilloscope (monitoring)

**Sample player avancé :**
```toml
[[plugins]]
id = "sample_player"
name = "Stage Sampler"
features = [
  "Multi-sample loading",
  "Velocity layers",
  "Pitch/speed control",
  "ADSR envelope",
  "Low-pass filter",
  "One-shot / loop modes",
  "Slice detection"
]
```

#### 4.4.3 Chaîne d'effets par piste

```toml
[[tracks]]
id = 3
name = "Vocal"
type = "audio_input"
input_channel = 0

# Chaîne de traitement (ordre = ordre d'exécution)
plugins = [
  { type = "highpass_filter", freq = 80 },
  { type = "compressor", threshold = -18, ratio = 3, attack = 5, release = 50 },
  { type = "eq3band", bass = 0, mid = 2, treble = 1 },
  { type = "reverb", type = "hall", size = 0.7, damping = 0.5, mix = 0.25 }
]

output = "main"
volume = 0.85
```

#### 4.4.4 Bus d'effets (sends)

Pour économiser CPU et créer cohérence sonore :

```toml
# Définir un bus d'effet
[[effect_buses]]
id = "reverb_bus"
plugins = [
  { type = "reverb", type = "hall", size = 0.8, mix = 1.0 }
]
output = "main"

# Envoyer plusieurs pistes vers ce bus
[[tracks]]
id = 1
name = "Snare"
sends = [
  { bus = "reverb_bus", amount = 0.4 }
]

[[tracks]]
id = 2
name = "Vocal"
sends = [
  { bus = "reverb_bus", amount = 0.6 }
]
```

#### 4.4.5 Plugins développés par la communauté

**API publique simplifiée :**

```rust
// Exemple de plugin simple en Rust
use stage_plugin_api::{Plugin, AudioBuffer, Parameters};

pub struct SimpleGain {
    gain: f32,
}

impl Plugin for SimpleGain {
    fn process(&mut self, input: &AudioBuffer, output: &mut AudioBuffer) {
        for (i, o) in input.iter().zip(output.iter_mut()) {
            *o = *i * self.gain;
        }
    }
    
    fn set_parameter(&mut self, id: &str, value: f32) {
        if id == "gain" {
            self.gain = value;
        }
    }
}
```

**Dossier de plugins tiers :**
```
~/.config/stage/plugins/
├── community/
│   ├── bitcrusher.sp
│   ├── granulator.sp
│   └── vocoder.sp
└── user/
    └── my_custom_fx.sp
```

#### 4.4.6 Contrôle des plugins en live

**Via TUI :**
```
┌─────────────────────────────────────────────────┐
│ Track 3: Vocal         [IN] -12dB  [OUT] -6dB  │
├─────────────────────────────────────────────────┤
│ [1] HPF        80Hz                             │
│ [2] Compressor -18dB  3:1  ●●●●●○○○ (active)   │
│ [3] EQ         B+0 M+2 T+1                      │
│ [4] Reverb     Hall 70%  Mix:25% ●●○○○○○○       │
├─────────────────────────────────────────────────┤
│ F1-F4: Toggle  Knob 1-8: Adjust  Enter: Edit   │
└─────────────────────────────────────────────────┘
```

**Via MIDI mapping :**
```toml
[midi_mappings]
# CC1 contrôle le reverb mix de la piste 3
[[mappings]]
controller = "Launch Control"
cc = 1
target = "track.3.plugin.reverb.mix"
min = 0.0
max = 1.0
```

### 4.5 Synchronisation

- **Master Clock interne** : BPM configurable
- **MIDI Clock** : In/Out, master/slave
- **Ableton Link** : Synchronisation réseau
- **Tap Tempo** : Ajustement manuel du BPM

---

## 5. Format de Projet

### 5.1 Structure de fichier
```
mon_set_live.stage/
├── project.toml          # Configuration principale
├── audio/               # Fichiers audio
│   ├── kick.wav
│   ├── bass.wav
│   └── pad.flac
├── samples/             # ⭐ Samples pour sample players
│   ├── drums/
│   │   ├── kick_01.wav
│   │   ├── snare_01.wav
│   │   └── hihat_01.wav
│   └── fx/
│       ├── riser.wav
│       └── impact.wav
├── midi/                # Séquences MIDI
│   ├── bass.mid
│   └── lead.mid
├── cv/                  # Courbes CV (format custom)
│   └── filter_env.cv
├── scenes/              # Définitions de scènes
│   ├── intro.toml
│   ├── verse.toml
│   └── chorus.toml
├── plugins/             # ⭐ Plugins custom du projet
│   ├── my_vocoder.sp
│   └── preset_reverb.toml
└── mappings/            # Contrôleurs
    ├── grid.toml
    └── midi_controller.toml
```

### 5.2 Format de fichier projet (project.toml)
```toml
[metadata]
version = "1.0"
created = "2026-01-26"
modified = "2026-01-26"
author = "Artist Name"

[project]
name = "Live Set Winter 2026"
bpm = 128
time_signature = "4/4"
master_volume = 1.0

[sync]
mode = "internal"  # internal, midi_clock, ableton_link
link_enabled = true

# ... reste de la config
```

---

## 6. Priorités de Développement

### Phase 1 : MVP (Minimum Viable Product)
- [ ] Engine de playback audio basique
- [ ] Séquenceur MIDI simple
- [ ] Interface TUI minimale
- [ ] Lecture de fichiers de projet TOML
- [ ] Sortie audio ALSA/JACK
- [ ] Sortie MIDI basique
- [ ] ⭐ Pistes audio input (capture basique)
- [ ] ⭐ Routing simple (input → output)

### Phase 2 : Performance Core
- [ ] Synchronisation précise audio/MIDI
- [ ] Optimisation latence
- [ ] Gestion de scènes/clips
- [ ] Contrôle clavier étendu
- [ ] Tests sur Raspberry Pi
- [ ] ⭐ Routing matriciel flexible
- [ ] ⭐ Plugin API de base

### Phase 3 : Plugins essentiels
- [ ] ⭐ EQ 3 bandes
- [ ] ⭐ Compresseur
- [ ] ⭐ Reverb simple
- [ ] ⭐ Delay tempo-sync
- [ ] ⭐ Gain/Pan utilitaires
- [ ] ⭐ Sample player basique
- [ ] ⭐ Chaîne de plugins par piste

### Phase 4 : Contrôleurs externes
- [ ] Support Monome Grid
- [ ] MIDI mapping configurable
- [ ] Protocole OSC
- [ ] Feedback visuel (LED)
- [ ] ⭐ Contrôle plugins via MIDI

### Phase 5 : Sorties avancées
- [ ] Support CV (Crow)
- [ ] Multi-sorties audio
- [ ] ⭐ Bus d'effets (sends/returns)
- [ ] ⭐ Re-sampling interne
- [ ] ⭐ Side-chaining

### Phase 6 : Environnement de préparation
- [ ] WebApp Tauri basique
- [ ] Éditeur de projet visuel
- [ ] Timeline drag & drop
- [ ] Export vers runtime
- [ ] ⭐ Éditeur de samples
- [ ] ⭐ Gestionnaire de plugins

### Phase 7 : Écosystème & Polish
- [ ] Plugin Ableton (optionnel)
- [ ] Documentation complète
- [ ] Presets et templates
- [ ] Communauté/partage de projets
- [ ] ⭐ Plugin SDK pour développeurs tiers
- [ ] ⭐ Support CLAP (plugins externes)

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
  - Carte son USB (optionnel, améliore latence)
- **Hardware recommandé** :
  - Raspberry Pi 4 (4GB)
  - Carte son USB classe compliant
  - Interface MIDI USB

### 7.3 Limites techniques
- **Pistes audio simultanées** : 16 minimum, 32 recommandé
- **Pistes MIDI simultanées** : 64
- **Sorties CV** : 8 canaux max
- **Taille fichier audio** : Illimitée (streaming)
- **Résolution audio** : 16/24-bit, 44.1-96kHz

---

## 8. Expérience Utilisateur

### 8.1 Workflow typique

#### Préparation (sur ordinateur)
1. Créer nouveau projet dans WebApp
2. Importer fichiers audio/MIDI
3. Organiser en scènes/pistes
4. Configurer routing des sorties
5. Mapper contrôleurs
6. Exporter projet (.stage)

#### Performance (sur Raspberry Pi/scène)
1. Copier projet sur Pi (USB/réseau)
2. Lancer : `stage run mon_set_live.stage`
3. Interface TUI s'affiche
4. Connecter contrôleurs (Grid, MIDI)
5. Performer : déclenchement scènes, mute/solo, contrôle paramètres
6. Logs sauvegardés pour post-mortem

### 8.2 Cas d'usage principaux

#### 1. DJ/Producer - Set live hybride
**Setup :**
- Backtracks audio (stems: drums, bass, pads)
- Synthé hardware connecté en audio input
- Effets en temps réel (reverb, delay, filter)
- Grid Monome pour déclenchement scènes

**Configuration :**
```toml
[[tracks]]
name = "Drums Stem"
type = "audio_playback"
file = "audio/drums.wav"
output = "main"

[[tracks]]
name = "Modular Synth"
type = "audio_input"
input_channel = 0
plugins = [
  { type = "lowpass_filter", cutoff = 2000, resonance = 0.7 },
  { type = "delay", time = "1/4", feedback = 0.4, mix = 0.3 }
]
sends = [{ bus = "reverb_bus", amount = 0.5 }]
output = "main"
```

#### 2. Live band électronique - Pistes + click
**Setup :**
- Pistes audio : backing vocals, synthés enregistrés
- Click track vers casque batteur (sortie dédiée)
- Sample player pour FX live (impacts, risers)
- MIDI vers synthé hardware

**Routing :**
```toml
[[outputs]]
id = "main"
channels = [0, 1]  # Public

[[outputs]]
id = "click"
channels = [2, 3]  # Casque musiciens

[[tracks]]
name = "Click Track"
type = "audio_playback"
file = "audio/click.wav"
output = "click"  # Isolé du mix principal

[[tracks]]
name = "FX Sampler"
type = "sample_player"
samples_dir = "samples/fx/"
midi_input = "LaunchPad"  # Déclenchement pads
output = "main"
```

#### 3. Performance solo - Looping live
**Setup :**
- Guitare/voix en audio input
- Sample player pour bases rythmiques
- Re-sampling pour créer loops en direct
- Effets de traitement créatif

**Workflow :**
1. Jouer guitare → chaîne d'effets → master
2. Pendant jeu, router guitare vers piste d'enregistrement
3. Nouvelle loop se déclenche en sync
4. Superposer plusieurs couches

```toml
[[tracks]]
name = "Guitar Input"
type = "audio_input"
plugins = ["compressor", "reverb"]
output = ["main", "loop_recorder"]  # Dual routing

[[tracks]]
name = "Loop 1"
type = "audio_playback"
file = "internal://recorded_loop_1"  # Enregistré en interne
loop = true
output = "main"
```

#### 4. Installation sonore interactive
**Setup :**
- Lecture autonome de scènes
- Déclenchement via OSC (capteurs, TouchOSC)
- Multi-sorties pour spatialisation
- Samples réactifs à des triggers externes

**Configuration OSC :**
```toml
[osc]
listen_port = 8000

[[osc_mappings]]
address = "/sensor/proximity"
target = "scene.next"  # Capteur déclenche scène suivante

[[osc_mappings]]
address = "/sensor/temperature"
target = "track.ambient.plugin.filter.cutoff"
min = 200
max = 5000  # Température contrôle filtre
```

#### 5. Prototype Eurorack/Hardware
**Setup :**
- CV output vers modules (via Crow)
- MIDI vers séquenceurs hardware
- Audio input pour capturer sorties Eurorack
- Analyse en temps réel (oscilloscope, spectre)

```toml
[[tracks]]
name = "CV Envelope"
type = "cv"
device = "crow"
channel = 1
curve = "adsr"
output = "modular_cv"

[[tracks]]
name = "Modular Return"
type = "audio_input"
input_device = "crow_audio"
plugins = ["analyzer", "eq"]  # Monitoring
output = "main"
```

#### 6. Podcast/Radio live
**Setup :**
- Microphones en audio input
- Samples de jingles, transitions
- Compresseur/EQ sur voix
- Multi-sorties (stream + casque + backup)

```toml
[[tracks]]
name = "Mic Host"
type = "audio_input"
plugins = [
  { type = "highpass_filter", freq = 80 },
  { type = "compressor", ratio = 3 },
  { type = "eq3band", bass = -2, mid = 2, treble = 1 }
]
output = ["stream", "monitor", "backup_recorder"]

[[tracks]]
name = "Jingles"
type = "sample_player"
samples_dir = "samples/jingles/"
output = ["stream", "monitor"]
```

---

## 9. Différenciation et Concurrence

### 9.1 Avantages vs solutions existantes

| Critère | STAGE | Ableton Live | Norns | Orca | Bitwig |
|---------|-------|--------------|-------|------|--------|
| Ressources | Très faible | Élevées | Moyen | Faible | Élevées |
| Latence | < 10ms | 10-20ms | < 10ms | Variable | 10-20ms |
| Audio+MIDI+CV | ✅ | ✅ | ✅ | âŒ (MIDI/OSC) | ✅ |
| Interface terminal | ✅ | âŒ | âŒ | ✅ | âŒ |
| Prix | Gratuit | â‚¬349+ | â‚¬650 | Gratuit | â‚¬399 |
| Orienté live | ✅ | ✅ | ✅ | âš ï¸ | ✅ |
| Modularité hardware | ✅ | âš ï¸ | ✅ | âš ï¸ | âš ï¸ |

### 9.2 Positionnement unique
- **Le seul** séquenceur terminal complet audio+MIDI+CV
- **Le plus léger** pour performance live professionnelle
- **Open source** et hackable
- **Conçu pour Raspberry Pi** dès le départ

---

## 10. Risques et Mitigation

### 10.1 Risques techniques
| Risque | Impact | Probabilité | Mitigation |
|--------|--------|-------------|------------|
| Latence audio trop élevée | Haut | Moyen | Utiliser JACK, optimiser buffers, tests sur Pi |
| Instabilité en live | Critique | Faible | Tests extensifs, mode safe, logs détaillés |
| Complexité TUI | Moyen | Élevé | Itération UX, modes simplifiés |
| Compatibilité hardware CV | Moyen | Moyen | Support limité à devices populaires (Crow) |

### 10.2 Risques projet
| Risque | Impact | Probabilité | Mitigation |
|--------|--------|-------------|------------|
| Scope creep | Élevé | Élevé | MVP strict, roadmap claire |
| Adoption limitée | Moyen | Moyen | Communauté early adopters, docs excellentes |
| Concurrence écosystème | Faible | Faible | Niche unique |

---

## 11. Roadmap Temporelle (Estimation)

### Trimestre 1 (Mois 1-3)
- Architecture de base
- Engine audio/MIDI
- TUI minimal
- Tests Raspberry Pi

### Trimestre 2 (Mois 4-6)
- Scènes/clips
- Synchronisation avancée
- Support Grid/OSC
- Alpha release

### Trimestre 3 (Mois 7-9)
- Support CV
- WebApp préparation
- Optimisations performance
- Beta release

### Trimestre 4 (Mois 10-12)
- Polish
- Documentation
- Communauté
- Release 1.0

---

## 12. Licencing et Open Source

### 12.1 Licence recommandée
**GPL v3** ou **MIT** selon philosophie :
- GPL : Garantit que dérivés restent open source
- MIT : Plus permissive, adoption plus large

### 12.2 Dépendances
Vérifier licences compatibles pour :
- CPAL (Apache 2.0)
- midir (MIT)
- ratatui (MIT)
- serde (MIT/Apache)

---

## 13. Communauté et Contribution

### 13.1 Canaux
- GitHub : Code, issues, discussions
- Discord/Forum : Entraide, partage de sets
- Documentation : Wiki, tutoriels vidéo

### 13.2 Contribution
- Guidelines de contribution claires
- Good first issues pour nouveaux contributeurs
- Exemples de projets .stage en repo

---

## 14. Monétisation (Optionnel)

Si le projet devient un produit :
- **Modèle dual** : Core open source, webapp payante
- **Services** : Hosting de projets, collaboration cloud
- **Hardware** : Kit Raspberry Pi pré-configuré
- **Formations** : Workshops, certifications

---

## 15. Conclusion

**STAGE** (nom recommandé) est positionné pour combler un vide dans l'écosystème : un séquenceur musical complet, léger, fiable, orienté performance live, fonctionnant sur hardware minimal. Son architecture modulaire et son approche pragmatique (préparation confortable, exécution efficace) en font un outil unique pour musiciens électroniques, installations sonores, et passionnés de hardware musical.

**Next steps :**
1. Valider spécifications avec utilisateurs potentiels
2. Prototyper playback audio basique en Rust
3. Tester latence sur Raspberry Pi 4
4. Concevoir format de fichier projet
5. Développer MVP TUI

---

**Document version** : 1.0
**Date** : 26 janvier 2026
**Auteur** : Spécification collaborative
