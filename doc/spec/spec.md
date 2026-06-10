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
|           RUNTIME DE PERFORMANCE                    |
|                                                     |
|  +---------------------------------------+          |
|  |    Terminal Sequencer (Rust)          |          |
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
|     | Audio   | | MIDI | | CV  |                    |
|     |  Out    | | Out  | | Out |                    |
|     +---------+ +------+ +-----+                    |
|                                                     |
|  +---------------------------------------+          |
|  |    Input Controllers                  |          |
|  |  • Terminal (TUI)                     |          |
|  |  • MIDI Controllers                   |          |
|  |  • Monome Grid                        |          |
|  +---------------------------------------+          |
+-----------------------------------------------------+
```

### 3.2 Technologies envisagées

#### Runtime de performance (Core)
- **Langage** : Rust
- **Audio** : `cpal` (abstraction cross-platform : ALSA/PipeWire, CoreAudio)
- **MIDI** : `midir`
- **TUI** : `ratatui`
- **Configuration** : `serde` + TOML ou YAML *(à trancher)*
- **Thread priorité** : `thread-priority`

#### Environnement de préparation
- **Option 1 - WebApp** : Tauri (Rust + HTML/CSS/JS)
- **Option 2 - Plugin Ableton** : Max4Live ou Python (extension future)

---

## 4. Fonctionnalités Détaillées

### 4.1 Gestion des pistes

#### Types de pistes

Une track est un **conduit de sortie**, défini une fois pour tout le projet. Elle ne contient pas de données musicales — le contenu vit dans les clips, à l'intérieur des sections de chaque song (cf. [data-model.md §2.2](data-model.md#22-track--un-conduit-de-sortie)).

1. **Pistes Audio Playback**
   - Format : WAV, FLAC (via `symphonia`)
   - Lecture temps réel avec buffer minimal
   - Contrôle volume/gain, état muet
   - Slot pour chaîne de plugins (voir §4.4)

2. **Pistes MIDI**
   - Fichiers `.mid` externes (MIDI standard), pas de séquences inlinées
   - Support clock MIDI externe *(à valider)*

3. **Pistes CV** (Control Voltage) *(hors MVP)*
   - Via interface externe (Monome Crow, Expert Sleepers, etc.)
   - Courbes de modulation
   - Gates/triggers
   - LFO intégrés

#### Organisation

Le projet suit une hiérarchie en 4 niveaux : **Project → Song → Section → Clip**, avec les tracks comme conduits transverses (cf. [data-model.md §2](data-model.md#2-hiérarchie)).

- Chaque **song** a son propre tempo (BPM) et sa signature rythmique
- Chaque **section** a une durée fixe (en mesures/beats) et un comportement de fin (`Advance`, `Stop`, `LoopFull`, `LoopTail`)
- Chaque **clip** est un pointeur vers un fichier externe (audio ou MIDI), joué sur une track dans une section

#### Routing audio

Chaque piste audio est routée vers une sortie physique. Le routage est direct : pas de bus internes ni de sends auxiliaires.

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

#### 4.2.3 MIDI Controller
- Mapping libre via fichier de configuration
- Support CC/Notes/Program Change


### 4.3 Sorties et Routing

#### Configuration modulaire

La configuration est séparée en deux fichiers distincts : la configuration du programme (audio backend, sorties physiques) et le fichier de projet (pistes, BPM, etc.).

**Configuration système (`config.toml` / `config.yaml`) :**
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

**Fichier de projet (`project.toml` / `project.yaml`) :**
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
clips = {
    1 = { type = "audio", file = "audio/kick_intro.wav" },
    2 = { type = "midi", file = "midi/bass.mid", playback = "loop" }
}

setlist = [1]
```

#### Types de sorties supportées
1. **Audio** : ALSA/PipeWire, JACK, CoreAudio
2. **MIDI** : Ports MIDI standard
3. **CV** : Monome Crow via USB série

### 4.4 Système de Plugins (architecture prévue, implémentation reportée)

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


### 4.5 Synchronisation

- **Master Clock interne** : tempo défini par song (cf. [data-model.md §2.3](data-model.md#23-song--une-chanson-du-setlist))
- **MIDI Clock** : In/Out, master/slave

---

## 5. Format de Projet

### 5.1 Structure de fichier
```
mon_set_live/
├── project.toml          # Configuration principale (format à trancher)
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

### 5.2 Format de fichier projet (TOML ou YAML, à trancher)
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
- [ ] Lecture de fichiers de projet (TOML ou YAML)
- [ ] Sortie audio ALSA/JACK
- [ ] Sortie MIDI basique
- [ ] Routing direct piste → sortie physique

### Phase 2 : Performance Core
- [ ] Synchronisation précise audio/MIDI
- [ ] Optimisation latence
- [ ] Gestion de scènes/sections
- [ ] Contrôle clavier étendu
- [ ] Tests sur Raspberry Pi

### Phase 3 : Contrôleurs externes
- [ ] Support Monome Grid
- [ ] MIDI mapping configurable
- [ ] Feedback visuel (LED)

### Phase 4 : Sorties avancées
- [ ] Support CV (Crow)
- [ ] Multi-sorties audio

### Phase 5 : Environnement de préparation
- [ ] WebApp Tauri basique
- [ ] Éditeur de projet visuel
- [ ] Timeline drag & drop
- [ ] Export vers runtime

### Phase 6 : Écosystème & Polish
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
- **Sorties CV** : 8 canaux max
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
clips = {
    1 = { type = "audio", file = "audio/drums.wav", playback = "loop" },
    2 = { type = "audio", file = "audio/bass.wav", playback = "loop" },
    3 = { type = "midi", file = "midi/lead.mid", playback = "loop" }
}
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
clips = {
    1 = { type = "audio", file = "audio/click.wav", playback = "loop" },
    2 = { type = "audio", file = "audio/bvox_v1.wav" }
}
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
clips = {
    2 = { type = "audio", file = "audio/drone.wav", playback = "loop" }
}
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
1. Prototyper le playback audio basique en Rust
2. Tester la latence sur Raspberry Pi 4
3. Trancher le format de fichier projet (TOML vs YAML)
4. Développer le MVP TUI

---

**Document version** : 1.2
**Date** : 25 avril 2026
**Auteur** : Spécification collaborative
