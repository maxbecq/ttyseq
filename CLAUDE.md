# CLAUDE.md

## Langue

- **Dialogue avec l'utilisateur : français.**
- **Code, commentaires, identifiants, messages de commit, documentation externe (README, doc publique) : anglais.**
- Les specs internes sous `doc/spec/` sont actuellement en français ; ne pas les traduire sans demande explicite.

## Vision

ttySeq est un séquenceur musical hybride **audio / MIDI / CV** en Rust, piloté au terminal,
conçu pour la **performance live** de musique électronique sur matériel à faibles ressources
(Raspberry Pi, vieux ordinateurs). C'est un **instrument de scène, pas un DAW** : la composition
se prépare ailleurs, ttySeq exécute de façon fiable et légère.

## État du projet

Phase de **spécification + tout début d'implémentation**. La doc précède le code.
Il n'y a pas encore de code source ni de découpage en crates matérialisé.
La référence fait foi : `doc/spec/spec.md` (vue d'ensemble) et `doc/spec/data-model.md`
(hiérarchie de données, comportements, décisions actées, questions ouvertes).

Lire ces deux fichiers avant toute proposition d'architecture ou de code.

## Architecture (cible)

- **Engine séquenceur** entouré d'une couche protocole ; un ou plusieurs **clients** s'y branchent
  (TUI embarqué, CLI, intégrations live coding). Voir `doc/spec/spec.md §3.3`.
- Vocabulaire de messages unique `EngineCommand` / `EngineEvent`, utilisé en interne (MPSC, sans
  sérialisation) comme en externe (socket Unix, OSC, avec `serde`). Garder les messages sous forme
  de **données pures** : pas de closures, pas de pointeurs partagés.
- Modes de lancement d'un binaire unique : `ttyseq` (engine + TUI embarqué, défaut solo),
  `ttyseq daemon` (headless), `ttyseq attach` (client TUI distant), `ttyseq <subcommand>` (one-shot CLI).
- Transports externes : socket Unix toujours ouvert (`$XDG_RUNTIME_DIR/ttyseq.sock`, pas de surface
  réseau) ; OSC sur UDP **désactivé par défaut**, loopback only quand activé.
- La sync tempo à l'échantillon (MIDI Clock, etc.) est un **sujet distinct** de cette IPC de contrôle.

## Règles temps réel (NON NÉGOCIABLES)

Dans le callback audio (chemin temps réel) :

- **Pas d'allocation** mémoire.
- **Pas de syscall.**
- **Pas de `panic!`** ni de `.unwrap()` / `.expect()` susceptibles de paniquer.
- Communication avec le reste du système via canaux et ring buffer lock-free (`rtrb`) uniquement.

Toute proposition qui viole une de ces règles dans le chemin audio doit être signalée et corrigée
avant d'aller plus loin.

## Conventions

- **Conventional Commits** (`feat:`, `fix:`, `docs:`, `refactor:`…).
- **Workflow par PR** : `main` est protégée, pas de push direct dessus.
- Maintenir le **numéro de version et la date** dans l'en-tête des specs lors d'un changement de fond.
- Lors d'une modification de spec, **nettoyer proactivement les références résiduelles**
  (commentaires, roadmap, exemples) qui pointent vers l'ancien état.

## Méthode de travail

- **Spike avant abstraction** : valider le chemin audio temps réel (cpal + ring buffer, 2 threads)
  avant de figer des interfaces inter-crates. Une mauvaise abstraction verrouillée tôt coûte cher.
- **Différer le découpage en crates** jusqu'à ce que les coutures émergent du code qui tourne.
- Itérer : valider les décisions dans le dialogue avant de les écrire dans les specs.
- Instructions d'édition courtes et précises attendues ; appliquer les changements de spec
  directement sur la branche de PR active.

## Points non tranchés (ne pas inventer de réponse)

À l'état actuel des specs, restent ouverts — ne pas présupposer un choix :

- **Licence** : GPL v3 vs MIT (cf. `spec.md §11`).
- **Format de config** : TOML vs YAML.
- **Format de sérialisation wire** externe : JSON vs MessagePack vs Bincode (cf. `spec.md §3.3.4`).
- Questions ouvertes du modèle de données : `doc/spec/data-model.md §6`
  (sémantique de Stop, modalités de Loop, offset des clips, transitions entre songs,
  policy de sample rate et de tempo des fichiers audio).

Si une décision sur un de ces points est nécessaire pour avancer, **le signaler explicitement**
et proposer des options, sans trancher unilatéralement.

## Stack technique

Rust. `cpal` (audio), `midir` (MIDI), `ratatui` (TUI), `rtrb` (ring buffer lock-free),
`thread-priority`, `serde`, `symphonia` (décodage audio). IPC externe : socket Unix natif,
OSC via `rosc` (à confirmer). Cibles : Raspberry Pi (dont Fates), macOS Apple Silicon (dev).
