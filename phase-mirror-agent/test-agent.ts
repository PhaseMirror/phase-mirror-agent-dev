import { PhaseMirrorAgent } from './index.js';
const agent = new PhaseMirrorAgent();
agent.analyze("test").then(console.log).catch(console.error);
