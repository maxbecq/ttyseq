# Spike : RME Babyface × Raspberry Pi

> Objectif : valider que la Babyface est utilisable comme interface audio de ttySeq
> sur Raspberry Pi — détection, multicanal, stabilité, latence. Timeboxé : si ça
> dépasse une demi-journée, on note où ça bloque et on repasse au code (le
> développement de l'engine ne dépend pas de ce résultat).

## Contexte et risques connus

- **Modèle concerné : Babyface Pro.**
- **Pas de driver RME sous Linux.** La Babyface ne fonctionne sur Pi qu'en mode
  **Class Compliant (CC)**. Sur la Babyface Pro, l'activation du mode CC se fait
  sur l'appareil lui-même, sans ordinateur (chapitre « Class Compliant Operation »
  du manuel RME — lire aussi ce qu'il dit du nombre de canaux et des sample rates
  exposés en CC, à confronter aux étapes 4-5 du protocole). Vérifier la version de
  firmware avant la session.
- **Pas de TotalMix sous Linux.** Le routing/mixage interne doit être soit
  pré-configuré, soit piloté par les contrôles en façade (selon modèle). À évaluer :
  est-ce compatible avec un usage scène sans ordinateur de config ?
- **Alimentation.** Le budget USB du Pi est limité ; une interface bus-powered peut
  être instable. Prévoir l'alimentation externe de la Babyface si le modèle le
  permet, ou un hub USB alimenté.
- ttySeq visera 48 kHz / buffer 512-1024 sur Pi (spec §7) : c'est la configuration
  à valider en priorité.

## Prérequis

- Modèle et firmware identifiés, mode CC activé (procédure faite depuis macOS).
- Raspberry Pi avec OS à jour ; `alsa-utils` installés.
- Le binaire du spike audio cpal (chemin temps réel déjà validé sur macOS).

## Protocole

Remplir la colonne résultat à chaque étape.

Session du 18/08/2026 — Pi 4 (1,8 Go, `gustave`), Debian 13 « trixie »,
noyau 6.18.39-rpt aarch64, Babyface Pro bus-powered directement sur le Pi.

| # | Étape | Commande / action | Attendu | Résultat |
|---|---|---|---|---|
| 1 | Détection | `aplay -l`, `cat /proc/asound/cards` | La Babyface apparaît comme carte ALSA | ✅ Carte 3 `Pro71993645` ; `lsusb` : `2a39:3fb0 RME Babyface Pro (Class Compliant Mode)` |
| 2 | MIDI | `amidi -l` | Les ports MIDI de la Babyface apparaissent | ✅ 2 ports IO : `hw:3,0,0` (Port 1), `hw:3,0,1` (Port 2) |
| 3 | Stéréo de base | `speaker-test -D hw:CARD=<nom> -c 2 -r 48000 -f S32_LE` | Son propre sur les sorties principales | ✅ Son propre confirmé au casque. Via `plughw` obligatoirement : le `hw` brut n'accepte que du S24_3LE, que `speaker-test` ne génère pas |
| 4 | Canaux exposés | `speaker-test -c <N>` en montant N | Nombre de canaux réellement accessibles en CC (noter : sorties casque/lignes/ADAT) | ✅ 12 out / 12 in (S24_3LE) d'après `/proc/asound/card3/stream0`. Vérifié à l'oreille (WAV 12 canaux, N bips sur canal N) : casque = canaux 3-4, recevant aussi en miroir les mains 1-2 (impairs à gauche, pairs à droite) ; canaux 5-12 = ADAT, non testés faute de matériel |
| 5 | Sample rates | tester 44.1 / 48 / 96 kHz | Rates acceptés en CC | ✅ Descripteur USB : 44.1 / 48 / 88.2 / 96 / 176.4 / 192 kHz. Lecture effective vérifiée à 48 kHz |
| 6 | Spike cpal | lancer le spike à buffer 1024, puis 512, 256, 128 | Pas de xrun audible ; noter le CPU (`top`) à chaque palier | ✅ Spike `spikes/babyface-cc/` (build debug), 48 kHz / 12 canaux, 60 s par palier : **0 underrun à 1024, 512, 256 et 128**. CPU ≈ 22-24 % d'un cœur à chaque palier. 0 xrun dmesg |
| 7 | Stress | lecture continue 30 min à la config cible (48 kHz / 512) | Zéro xrun (`dmesg`, compteurs ALSA) | ✅ **30 min / 0 underrun** (48 kHz / 512 / 12 canaux, build debug), CPU stable ≈ 25 % d'un cœur, 0 xrun dmesg |
| 8 | Hotplug | débrancher/rebrancher en cours de route | La carte réapparaît proprement (comportement à documenter pour la spec hotplug §3.3.7) | ✅ Débranchement en cours de lecture : le processus survit (pas de panic), le callback d'erreur cpal remonte proprement « buffer underrun » puis « device not available » puis « Device disconnected ». Rebranchement : la carte réapparaît au même index ALSA (carte 3) ; warnings dmesg bénins (« falling back to MIDI 1.0 », « unit 2 not found »). **Le flux cpal ne reprend pas tout seul : l'application doit détecter la disparition et rouvrir le device** — à intégrer dans la spec hotplug §3.3.7 |
| 9 | (Option) Latence round-trip | JACK + `jack_iodelay` avec câble loopback sortie→entrée | Mesure réelle, à comparer à l'objectif < 10 ms | ⏸️ Différée : pas de câble de loopback disponible pendant la session |

## Critères de succès

Le spike est concluant si : détection fiable, **au moins 4 canaux de sortie**
utilisables (stéréo main + click séparé), 48 kHz / buffer ≤ 512 stable 30 min
sans xrun sur le spike cpal.

## Sorties du spike

- Grille ci-dessus remplie, versionnée dans ce fichier.
- Décision : la Babyface est-elle l'interface de référence sur Pi, ou faut-il
  prévoir/recommander une alternative class-compliant ?
- Retombées spec éventuelles : contraintes CC à documenter (canaux, routing sans
  TotalMix), comportement hotplug observé.

## Bilan de la session du 18-19/08/2026

**Tous les critères de succès sont atteints** : détection fiable, 12 canaux de
sortie exposés (≥ 4 requis ; 4 analogiques vérifiés à l'oreille, 8 ADAT non
testés faute de matériel), 48 kHz / buffer 512 stable 30 min sans le moindre
underrun (et même buffer 128 stable 60 s), en **bus-powered directement sur le
Pi 4** — le risque alimentation ne s'est pas matérialisé sur ce setup.

Proposition (à valider) : **la Babyface Pro en mode CC est utilisable comme
interface de référence de ttySeq sur Pi.**

Contraintes CC relevées, à documenter côté spec :

- Format natif S24_3LE uniquement sur le `hw` brut ; passer par `plughw` pour
  du f32 cpal (conversion ALSA, surcoût négligeable ici).
- Casque sur les canaux 3-4, recevant aussi en miroir les mains 1-2 — routing
  interne figé par l'appareil, pas de TotalMix sous Linux.
- Hotplug : la carte réapparaît au même index, mais le flux cpal est
  irrécupérable — l'engine devra détecter la déconnexion (callback d'erreur)
  et rouvrir le device (spec §3.3.7).
- Latence round-trip non mesurée (étape 9 différée, câble manquant).
