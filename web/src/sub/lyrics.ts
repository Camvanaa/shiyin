import { mount } from 'svelte';
import '../app.css';
import Lyrics from './Lyrics.svelte';

mount(Lyrics, { target: document.getElementById('app')! });
