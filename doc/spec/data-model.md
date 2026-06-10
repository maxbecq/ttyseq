# Modèle de données ttySeq

> Document de spécification pour `ttyseq-core` — la modélisation des données qui décrivent un projet de live.
> 
> Ce document est en cours de finalisation. Les décisions actées sont marquées ✅. Les questions ouvertes sont marquées ❓ et appellent une décision avant l'implémentation.

---

## 1. Philosophie

ttySeq est un **outil de live**, pas un outil de composition. Il **assemble et exécute** ; il ne crée pas le contenu musical.

Concrètement :

- L'audio est préparé dans un DAW (Ableton, REAPER, etc.) et exporté en WAV/FLAC
- Les séquences MIDI sont préparées ailleurs et exportées en `.mid` (format MIDI standard)
- ttySeq orchestre l'enchaînement de ces assets pendant le live

Le projet ttySeq est donc une **partition de session**, pas un conteneur de données musicales. Il est léger, lisible, versionnable.

## 2. Hiérarchie

Quatre niveaux, du plus large au plus fin :

```
Project
├── tracks               (les "conduits" — fixes pour tout le live)
├── songs                (catalogue de chansons)
│   └── Song
│       └── sections     (les "scènes" qui composent la chanson)
│           └── Section
│               └── clips   (un par track active dans la section)
│                   └── Clip (Audio | Midi | Empty)
└── setlist              (ordre des songs dans le live)
```

### 2.1 Project — la racine

Un fichier projet décrit **un live entier** : tout ce qui est nécessaire pour exécuter une performance, du début à la fin.

Contenu :

- **Métadonnées** : nom, auteur, dates de création/modification, version du format
- **Tracks** : les conduits de sortie, fixes pour toute la durée du live (cf. §2.2)
- **Songs** : le catalogue des chansons disponibles
- **Setlist** : la liste ordonnée des songs à jouer

### 2.2 Track — un conduit de sortie

Une track représente **une voie indépendante de sortie**. Elle est définie une fois pour tout le projet et ne change pas d'une song à l'autre.

Une track a :

- Un type : **Audio** ou **MIDI** (le **CV** est un placeholder architectural, hors MVP)
- Un nom
- Une cible de sortie (canal de carte son, port MIDI…)
- Un volume / gain
- Un état muet
- Un emplacement plugin (placeholder, vide en MVP)

**Important** : la track est un conduit, pas un container de contenu musical. Le contenu musical vit dans les clips à l'intérieur des sections.

### 2.3 Song — une chanson du setlist

Une song correspond à un morceau du live. Elle a son propre tempo et sa propre signature rythmique.

Une song contient :

- Un nom
- Un tempo (BPM)
- Une signature rythmique (4/4, 3/4…)
- Une liste **ordonnée** de sections

### 2.4 Section — l'unité fondamentale du live

Une section est une **scène musicale** : un moment de la song où certaines tracks jouent certains clips, pour une certaine durée, avec un certain comportement à la fin.

Une section a :

- Un nom (ex. "Intro", "Couplet 1", "Refrain")
- Une **longueur musicale** (`length`) — exprimée en mesures ou beats
- Un **comportement de fin** (`on_end`) — voir §3
- Une **collection de clips**, indexée par track : à chaque track active dans cette section correspond un clip

**Une track sans clip dans une section est silencieuse pour cette section** (équivalent à `Empty`).

### 2.5 Clip — le contenu musical à jouer

Un clip est un pointeur vers du contenu musical externe, à jouer dans une section sur une track donnée.

Trois variantes :

- **AudioClip** : référence un fichier audio (WAV/FLAC), avec gain et mode de lecture
- **MidiClip** : référence un fichier `.mid`, avec transposition et mode de lecture
- **Empty** : explicite l'absence de contenu (équivalent à un clip muet)

**Modes de lecture** d'un clip (`OneShot` ou `Loop`) :

- `OneShot` : joue une fois, puis silence si la section dure plus longtemps
- `Loop` : boucle si plus court que la section

**Si un clip dépasse la durée de la section : il est coupé net.** ✅ (cf. §6.3)

## 3. Comportements de fin de section

Quatre comportements possibles, exprimés par `SectionBehavior` :

| Valeur | Comportement |
|---|---|
| `Advance` | À la fin de la section, **passer automatiquement** à la section suivante (ou à la song suivante si c'est la dernière section). |
| `Stop` | À la fin de la section, **arrêter la lecture**. Le performeur appuie sur Play pour continuer (à la section suivante). |
| `LoopFull` | **Boucler la section entière** indéfiniment jusqu'à ce que le performeur appuie sur Stop. |
| `LoopTail { tail_length }` | Jouer la section une fois, puis **boucler les N derniers beats** jusqu'au Stop. |

**Une seule commande de transport : Play/Stop.** ✅

Le performeur n'a pas de touches dédiées "section suivante" ou "song suivante". L'enchaînement est entièrement piloté par les `on_end` écrits dans le projet, et les boucles sont interrompues par Stop.

## 4. Schéma de l'exécution

```
[Play] →  Song[0] → Section[0] → fin selon on_end →
                  → Section[1] → fin selon on_end →
                  → ...
                  → Section[N] → fin selon on_end →
       →  Song[1] → Section[0] → ...
       →  ...
       →  Song[M] → Section[N] → [fin du live]
```

À chaque section, le moteur :

1. Démarre tous les clips associés (un par track) en simultané
2. Compte les beats jusqu'à atteindre `length`
3. Applique `on_end` :
   - `Advance` → enchaîne sur la section suivante
   - `Stop` → arrête, attend Play
   - `LoopFull` → revient au début de la section, attend Stop
   - `LoopTail` → revient au point `length - tail_length`, attend Stop

## 5. Décisions actées ✅

1. **Hiérarchie en 4 niveaux** : Project → Song → Section → Clip, avec les Tracks comme conduits transverses.
2. **Tempo par song**, signature rythmique par song.
3. **Sections de durée fixe** : la durée est une propriété explicite de la section, indépendante du contenu des clips.
4. **Clips coupés net** s'ils dépassent la durée de la section.
5. **Une seule commande de transport** Play/Stop, qui interagit avec `on_end` pour produire tous les comportements.
6. **Quatre comportements de fin de section** : `Advance`, `Stop`, `LoopFull`, `LoopTail`.
7. **Pas de saut arbitraire entre sections en live (MVP).** L'avancement est strictement séquentiel.
8. **MIDI = fichiers .mid externes**, pas de step sequencer interne, pas d'événements MIDI inlinés dans le projet.
9. **Audio = fichiers WAV/FLAC externes**, référencés par chemin relatif.
10. **Pas de probabilité ni de variations** sur les notes ou les clips (hors MVP).

## 6. Questions ouvertes ❓

Ces points doivent être tranchés avant de figer les types Rust de `ttyseq-core`.

### 6.1 Sémantique exacte de Stop

Quand le performeur appuie sur Stop pendant la lecture, deux scénarios possibles :

- **Stop immédiat** : la lecture s'arrête net (sample-accurate). Re-Play reprend où ?
  - Au début de la section courante ?
  - Au début de la song ?
  - À la section suivante ?
- **Stop "fin de section"** : la lecture continue jusqu'à la fin de la section courante puis s'arrête. Re-Play reprend à la section suivante (sauf si la section était `Stop`, alors à la même).

Pour du live électronique, **Stop immédiat avec reprise à la section suivante** semble le plus naturel : "je coupe parce que ça dérape, et je relance sur la suite". Mais à confirmer.

### 6.2 Modalités de Loop

Pour `LoopFull` et `LoopTail` :

- Nombre de répétitions max : illimité jusqu'au Stop, ou compteur explicite ?
- Pour `LoopTail`, le paramètre est-il `tail_length` (durée de la queue à boucler) ou `tail_start` (point dans la section où la boucle redémarre) ? Les deux représentations sont équivalentes mais l'une est peut-être plus intuitive.

### 6.3 Offset des clips dans une section

Un clip peut-il commencer **plus tard** que le début de la section ? Cas d'usage : faire entrer un instrument 4 beats après le début d'une section.

- **Si oui** : ajouter un champ `start_offset: MusicalDuration` au clip
- **Si non** : tous les clips démarrent strictement au début de la section ; pour décaler une entrée, on découpe en deux sections successives

Le second est plus simple et probablement suffisant en MVP.

### 6.4 Transitions entre songs

Quand une song termine sa dernière section :

- La song suivante du setlist démarre-t-elle automatiquement ?
- Le tempo change-t-il instantanément, ou y a-t-il une transition ?
- Faut-il un comportement explicite "fin de song" (équivalent du `SectionBehavior` mais au niveau song) ?

Ma proposition : la dernière section d'une song dicte le comportement, via son `on_end`. Une song qui doit s'enchaîner en automatique se termine par une section `Advance` ; une song qui doit attendre une décision se termine par `Stop`.

### 6.5 Sample rate des fichiers audio

Si la carte son tourne à 48 kHz et qu'un fichier source est à 44.1 kHz :

- **Resampling à la volée** : ttySeq adapte automatiquement (plus user-friendly)
- **Exigence de match** : ttySeq refuse de charger ; le performeur convertit ses fichiers en amont (plus simple, plus performant, pas d'ambiguïté)

Pour un MVP orienté performance et fiabilité, l'exigence de match semble préférable. Le performeur prépare son live, il sait à quelle fréquence il joue.

### 6.6 Tempo des fichiers audio

Si la song est à 128 BPM mais le fichier audio a été enregistré à 130 BPM :

- **Time-stretching à la volée** : très complexe, coûteux, qualité variable
- **Exigence de match** : le performeur prépare son fichier au bon tempo

**Pas de time-stretching en MVP** semble évident. Le performeur cale ses fichiers en amont.

## 7. Hors scope MVP (pour mémoire)

Pour rappel, ces fonctionnalités ont été explicitement écartées du MVP :

- Saut arbitraire entre sections en live (grid de navigation TUI) — *intéressant, mais plus tard*
- Probabilité / variations sur les notes — *intéressant, mais plus tard*
- Time-stretching audio — *complexité disproportionnée*
- Resampling à la volée — *à reconsidérer après §6.5*
- Plugins — *placeholder architectural uniquement*
- CV output — *architectural, mais pas implémenté en MVP*
- Bus internes, effets, sends auxiliaires — *écarté définitivement*
- Matrix routing — *direct track-to-output uniquement*
- Tap tempo, Ableton Link, MIDI learn — *écarté définitivement*

## 8. Représentation textuelle d'un projet (illustratif)

Pour donner une intuition concrète, voici à quoi pourrait ressembler un projet ttySeq exprimé en TOML — illustratif, non normatif à ce stade :

```toml
[metadata]
name = "Live au Petit Bain"
author = "Max"

[[tracks]]
id = 1
name = "Drums"
type = "audio"
output = { type = "interface_channel", channel = 1 }

[[tracks]]
id = 2
name = "Bass"
type = "midi"
output = { type = "device", name = "Crow" }
channel = 1

[[songs]]
id = 1
name = "Opener"
tempo = 128.0
time_signature = [4, 4]

[[songs.sections]]
name = "Intro"
length = { bars = 8 }
on_end = "advance"
clips = { 1 = { type = "audio", file = "drums/intro.wav" } }

[[songs.sections]]
name = "Drop"
length = { bars = 16 }
on_end = { type = "loop_tail", tail_length = { bars = 4 } }
clips = {
    1 = { type = "audio", file = "drums/drop.wav", playback = "loop" },
    2 = { type = "midi", file = "bass/drop.mid", playback = "loop" }
}

[[songs.sections]]
name = "Outro"
length = { bars = 4 }
on_end = "stop"
clips = { 1 = { type = "audio", file = "drums/outro.wav" } }

setlist = [1]
```

Cette représentation reste indicative — la syntaxe exacte sera affinée au moment d'implémenter `ttyseq-project`.

---

*Dernière mise à jour : avril 2026. Les questions §6 doivent être tranchées avant l'implémentation des types Rust de `ttyseq-core`.*
