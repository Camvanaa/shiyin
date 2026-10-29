import { mount } from 'svelte';
import '../app.css';
import Mini from './Mini.svelte';

mount(Mini, { target: document.getElementById('app')! });
