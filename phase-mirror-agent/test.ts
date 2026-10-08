import { pipeline } from '@huggingface/transformers';

async function main() {
  const generator = await pipeline('text-generation', 'Xenova/Qwen1.5-0.5B-Chat');
  const messages = [
    { role: 'system', content: 'You are an AI.' },
    { role: 'user', content: 'Hello' }
  ];
  const result = await generator(messages, {
    max_new_tokens: 20,
    temperature: 0.7,
    do_sample: true,
    return_full_text: false
  });
  console.log("RESULT:", JSON.stringify(result, null, 2));
}

main();
