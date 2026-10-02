"""Original local instrumental: deterministic synthesis, no samples or services."""
from pathlib import Path
import numpy as np
import soundfile as sf

sr, duration, bpm = 48000, 70, 128
beat = 60 / bpm
mix = np.zeros((sr * duration, 2), dtype=np.float64)
rng = np.random.default_rng(20261001)

def add(signal, start, gain=1, pan=0):
    offset = round(start * sr)
    count = min(len(signal), len(mix) - offset)
    if count <= 0: return
    mix[offset:offset+count, 0] += signal[:count] * gain * np.sqrt((1-pan)/2)
    mix[offset:offset+count, 1] += signal[:count] * gain * np.sqrt((1+pan)/2)

def note(midi, seconds, kind='pluck'):
    t = np.arange(round(seconds*sr))/sr
    f = 440*2**((midi-69)/12)
    if kind == 'pad':
        wave = np.sin(2*np.pi*f*t) + .3*np.sin(2*np.pi*(f*1.003)*t) + .12*np.sin(4*np.pi*f*t)
        envelope = np.minimum(t/.12,1)*np.minimum((seconds-t)/.25,1)
    else:
        wave = np.sin(2*np.pi*f*t) + .35*np.sin(4*np.pi*f*t) + .12*np.sin(6*np.pi*f*t)
        envelope = (1-np.exp(-t*650))*np.exp(-t*(7 if kind=='pluck' else 3))
    return wave*envelope

chords = [(50,54,57,61), (57,61,64,69), (59,62,66,69), (55,59,62,66)]
for b in range(int(duration/beat)+1):
    start=b*beat
    chord=chords[(b//4)%4]
    energy= .65 if 54 <= start < 60 else 1
    t=np.arange(round(.3*sr))/sr
    phase=2*np.pi*(48*t+110*.045*(1-np.exp(-t/.045)))
    add(np.sin(phase)*np.exp(-t*18),start,.34*energy)
    if b%2:
        t=np.arange(round(.15*sr))/sr
        noise=rng.normal(size=len(t))
        snare=(noise-.8*np.roll(noise,1))*np.exp(-t*27)
        add(snare,start,.045*energy)
    for half in [0,.5]:
        t=np.arange(round(.065*sr))/sr
        noise=rng.normal(size=len(t))
        hat=(noise-np.roll(noise,1))*np.exp(-t*75)
        add(hat,start+half*beat,.016*energy,(-1 if b%2 else 1)*.3)
    add(note(chord[0]-12,beat*.9,'bass'),start,.15)
    for eighth in range(4):
        midi=chord[(b+eighth)%4]+12
        pluck=note(midi,beat*.8)
        when=start+eighth*beat/4
        add(pluck,when,.06*energy,(eighth-1.5)*.25)
        add(pluck,when+.1875,.018*energy,-(eighth-1.5)*.25)
    if b%4==0:
        for midi in chord: add(note(midi,beat*4,'pad'),start,.023)

mix=np.tanh(mix*1.3)
mix*=np.minimum(np.arange(len(mix))/sr/.15,1)[:,None]
mix*=np.minimum((duration-np.arange(len(mix))/sr)/1.2,1)[:,None]
mix*=.85/max(np.max(np.abs(mix)),1e-8)
output=Path(__file__).parent/'assets/audio/launch-original.wav'
output.parent.mkdir(parents=True,exist_ok=True)
sf.write(output,mix,sr,subtype='PCM_24')
print(output)
