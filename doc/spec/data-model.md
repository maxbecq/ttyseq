# Modèle de données ttySeq

> Document de spécification pour `ttyseq-core` — la modélisation des données qui décrivent un projet de live.
> 
> Les décisions actées sont marquées ✅. Toutes les questions ouvertes ont été tranchées (18 août 2026) : ce document décrit l'état acté du modèle de données, récapitulé en §5.

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

**Si un clip dépasse la durée de la section : il est coupé net.** ✅

**Tous les clips d'une section démarrent au début de la section** — pas de `start_offset` en MVP ; une entrée décalée se modélise en découpant la section en deux. ✅

## 3. Comportements de fin de section

Quatre comportements possibles, exprimés par `SectionBehavior` :

| Valeur | Comportement |
|---|---|
| `Advance` | À la fin de la section, **passer automatiquement** à la section suivante (ou à la song suivante si c'est la dernière section). |
| `Stop` | À la fin de la section, **arrêter la lecture**. Le performeur appuie sur Play pour continuer (à la section suivante). |
| `LoopFull` | **Boucler la section entière** indéfiniment, jusqu'à une sortie de boucle demandée par Stop (cf. §3.1). |
| `LoopTail { tail_length }` | Jouer la section une fois, puis **boucler les `tail_length` derniers beats** indéfiniment, jusqu'à une sortie demandée par Stop. `tail_length` est une durée relative à la fin de la section. |

Les boucles sont **illimitées** : pas de compteur de répétitions — c'est le performeur qui décide quand avancer. ✅

**Une seule commande de transport : Play/Stop.** ✅

Le performeur n'a pas de touches dédiées "section suivante" ou "song suivante". L'enchaînement est entièrement piloté par les `on_end` écrits dans le projet, et les sorties de boucle sont demandées par Stop.

### 3.1 Sémantique de Stop ✅

L'effet de Stop dépend du contexte de lecture :

- **Hors boucle** : Stop **coupe net** (sample-accurate). Un Play ultérieur reprend **au début de la section suivante** (ou de la song suivante si c'était la dernière section de la song). Usage : « je coupe parce que ça dérape, je relance sur la suite. »
- **Pendant une boucle** (`LoopFull` ou `LoopTail` en cours) : le premier Stop **programme la sortie de boucle** — le passage en cours se termine, puis la lecture enchaîne sur la section suivante, sans silence. Un **second Stop** pendant ce dernier passage **coupe net** (cas d'urgence), avec la même sémantique de reprise que hors boucle.

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
   - `LoopFull` → revient au début de la section ; un Stop programme la sortie en fin de passage (cf. §3.1)
   - `LoopTail` → revient au point `length - tail_length` ; sortie demandée de la même façon par Stop

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
11. **Sémantique de Stop hybride** (cf. §3.1) : hors boucle, coupe nette avec reprise à la section suivante ; pendant une boucle, sortie musicale en fin de passage, un second Stop coupant net.
12. **Boucles illimitées jusqu'au Stop** — pas de compteur de répétitions. `LoopTail` est paramétré par `tail_length` (durée de la queue, relative à la fin de la section — reste valide si la section change de longueur).
13. **Pas de `start_offset` sur les clips (MVP)** : tout clip démarre au début de sa section ; une entrée décalée se modélise en découpant la section en deux. Un champ optionnel restera ajoutable plus tard sans casser le format.
14. **Fin de song dictée par le `on_end` de sa dernière section** : `Advance` enchaîne automatiquement sur la song suivante du setlist (changement de tempo et de signature instantané à la frontière), `Stop` attend Play. Pas de mécanisme dédié au niveau song.
15. **Sample rate : exigence de match** avec la carte son. Refus au chargement du projet (jamais en cours de live) avec message explicite — fichier, rate attendu, rate trouvé. Pas de resampling à la volée en MVP.
16. **Pas de time-stretching** : les fichiers audio sont calés au tempo de la song en amont.
17. **Format du fichier projet : TOML** (également pour la configuration système, cf. [spec.md §4.3](spec.md#43-sorties-et-routing)).

## 6. Hors scope MVP (pour mémoire)

Pour rappel, ces fonctionnalités ont été explicitement écartées du MVP :

- Saut arbitraire entre sections en live (grid de navigation TUI) — *intéressant, mais plus tard*
- Probabilité / variations sur les notes — *intéressant, mais plus tard*
- Time-stretching audio — *complexité disproportionnée*
- Resampling à la volée — *écarté du MVP au profit de l'exigence de match (cf. §5, décision 15) ; reconsidérable ensuite*
- Offset de départ des clips dans une section (`start_offset`) — *écarté du MVP (cf. §5, décision 13)*
- Plugins — *placeholder architectural uniquement*
- CV output — *architectural, mais pas implémenté en MVP*
- Track/clip de type « live » (flux d'événements externes timestampés, instrument live coding) — *documenté dans [spec.md §10](spec.md#10-risques-et-mitigation)*
- Bus internes, effets, sends auxiliaires — *écarté définitivement*
- Matrix routing — *direct track-to-output uniquement*
- Tap tempo, Ableton Link, MIDI learn — *écarté définitivement*

## 7. Représentation textuelle d'un projet (illustratif)

Le format du fichier projet est le **TOML** ✅. L'exemple ci-dessous illustre la syntaxe — la grammaire exacte sera affinée en implémentant le parsing. Noter la contrainte TOML : les tables inline tiennent sur une seule ligne, d'où la sous-table `[songs.sections.clips]` quand une section a plusieurs clips :

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

[songs.sections.clips]
1 = { type = "audio", file = "drums/drop.wav", playback = "loop" }
2 = { type = "midi", file = "bass/drop.mid", playback = "loop" }

[[songs.sections]]
name = "Outro"
length = { bars = 4 }
on_end = "stop"
clips = { 1 = { type = "audio", file = "drums/outro.wav" } }

setlist = [1]
```

Cette représentation reste indicative — la syntaxe exacte sera affinée au moment d'implémenter le parsing du projet.

---

*Dernière mise à jour : 18 août 2026. Toutes les questions ouvertes ont été tranchées ; les décisions sont récapitulées en §5. Les types Rust de `ttyseq-core` peuvent être figés.*
