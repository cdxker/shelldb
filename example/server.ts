import express from 'express';
import type { Request, Response } from 'express';
import cors from 'cors';
import Anthropic from '@anthropic-ai/sdk';

// Types
type FileToIndex = {
  name: string;
  content: string;
  tags?: string[];
};

type UpdateWorkflowRequest = {
  dataset_uuid: string;
  files: FileToIndex[];
};

type AssistantRequest = {
  dataset_uuid: string;
  message: string;
};

// Environment variables
const GREPDB_URL = Bun.env.GREPDB_URL || 'http://localhost:8080';
// Hardcoded for local example - normally would use env variable
const ANTHROPIC_API_KEY = Bun.env.ANTHROPIC_API_KEY;
const PORT = parseInt(Bun.env.PORT || '3002');

// Initialize Express app
const app = express();

// Configure CORS - completely permissive
app.use(cors({
  origin: '*',  // Allow all origins
  methods: ['GET', 'POST', 'PUT', 'DELETE', 'OPTIONS', 'HEAD'],  // Allow all methods
  allowedHeaders: '*',  // Allow all headers
  credentials: true,  // Allow credentials
  maxAge: 86400  // Cache preflight response for 24 hours
}));

app.use(express.json());

// Initialize Anthropic client
const anthropic = new Anthropic({
  apiKey: ANTHROPIC_API_KEY,
});

// Grep function for searching in GrepDB
async function grep(dataset_uuid: string, queryFlags: string): Promise<string> {
  try {
    const requestBody = {
      dataset: dataset_uuid,
      flags: queryFlags,
      folder_filter: [],  // Empty array = search all folders
    };
    console.log('Sending to GrepDB:', JSON.stringify(requestBody));

    const response = await fetch(`${GREPDB_URL}/api/search`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(requestBody),
    });

    if (!response.ok) {
      throw new Error(`GrepDB search failed: ${response.status} ${response.statusText}`);
    }

    const result = await response.text();
    return result;
  } catch (error) {
    console.error('Error calling GrepDB search:', error);
    throw error;
  }
}

// Assistant message handler
async function assistant_message_handler(req: Request, res: Response) {
  try {
    const { dataset_uuid, message } = req.body as AssistantRequest;

    if (!dataset_uuid || !message) {
      return res.status(400).json({ error: 'Missing dataset_uuid or message' });
    }

    // Prepare the system prompt with grep tool description
    const systemPrompt = `You are an AI assistant with access to a grep search tool for searching through indexed files.
You can use the grep function to search for patterns in the dataset.
When the user asks you to search for something, use the grep tool with appropriate flags.
Common grep flags include:
- -i for case-insensitive search
- -r for recursive search
- -n to show line numbers
- -c to show count of matches
- -v to invert match (show lines that don't match)

To use the grep tool, respond with a tool use in this exact format:
<grep_search>FLAGS PATTERN</grep_search>

For example:
<grep_search>-i error</grep_search>
<grep_search>-r -n function</grep_search>

Dataset UUID: ${dataset_uuid}`;

    // Create a stream for the response
    res.setHeader('Content-Type', 'text/event-stream');
    res.setHeader('Cache-Control', 'no-cache');
    res.setHeader('Connection', 'keep-alive');
    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('X-Accel-Buffering', 'no');

    // Send initial message to Claude
    const stream = await anthropic.messages.create({
      model: 'claude-3-5-sonnet-20241022',
      max_tokens: 4096,
      system: systemPrompt,
      messages: [
        {
          role: 'user',
          content: message,
        },
      ],
      stream: true,
    });

    let fullResponse = '';
    const executedCommands = new Set<string>();

    for await (const chunk of stream) {
      if (chunk.type === 'content_block_delta' && chunk.delta.type === 'text_delta') {
        const text = chunk.delta.text;
        fullResponse += text;

        // Send the text chunk to the client
        res.write(`data: ${JSON.stringify({ type: 'text', content: text })}\n\n`);

        // Check if we have a complete grep command
        const grepMatch = fullResponse.match(/<grep_search>(.*?)<\/grep_search>/);
        if (grepMatch && grepMatch[1]) {
          const grepCommand = grepMatch[1];
          const commandKey = `grep:${grepCommand}`;

          // Only execute if we haven't already executed this command
          if (!executedCommands.has(commandKey)) {
            executedCommands.add(commandKey);

            // Execute grep search
            try {
              console.log(`Executing grep command: "${grepCommand}"`);
              const grepResult = await grep(dataset_uuid!, grepCommand);

              // Send grep results to client
              res.write(`data: ${JSON.stringify({ type: 'grep_result', content: grepResult })}\n\n`);
            } catch (grepError: any) {
              console.error(`Grep error for command "${grepCommand}": ${grepError.message}`);
              res.write(`data: ${JSON.stringify({ type: 'error', content: `Grep error: ${grepError.message}` })}\n\n`);
            }
          }
        }
      }
    }

    // Send completion signal
    res.write(`data: ${JSON.stringify({ type: 'done' })}\n\n`);
    res.end();
  } catch (error: any) {
    console.error('Assistant error:', error);
    res.status(500).json({ error: error.message });
  }
}

// Update index handler
async function update_index_handler(req: Request, res: Response) {
  try {
    const { dataset_uuid, files } = req.body as UpdateWorkflowRequest;

    if (!dataset_uuid) {
      return res.status(400).json({ error: 'Missing dataset_uuid' });
    }

    if (!files || !Array.isArray(files)) {
      return res.status(400).json({ error: 'Missing or invalid files array' });
    }

    const results = [];
    let successCount = 0;
    let errorCount = 0;

    for (const file of files) {
      try {
        const indexResponse = await fetch(`${GREPDB_URL}/api/index`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            dataset: dataset_uuid,
            filename: file.name,
            file_payload: file.content,
            nested: file.tags,
          }),
        });

        if (indexResponse.ok) {
          successCount++;
          results.push({ file: file.name, status: 'success' });
        } else {
          errorCount++;
          const errorText = await indexResponse.text();
          results.push({ file: file.name, status: 'error', error: errorText });
        }
      } catch (error: any) {
        errorCount++;
        results.push({ file: file.name, status: 'error', error: error.message });
      }
    }

    res.json({
      message: `Indexed ${successCount} files successfully, ${errorCount} errors`,
      total: files.length,
      success: successCount,
      errors: errorCount,
      details: results,
    });
  } catch (error: any) {
    console.error('Update index error:', error);
    res.status(500).json({ error: error.message });
  }
}

// Routes
app.post('/assistant', assistant_message_handler);
app.post('/update_workflow', update_index_handler);

// Health check endpoint
app.get('/health', (_: Request, res: Response) => {
  res.json({ status: 'ok', grepdb_url: GREPDB_URL });
});

// Start server
const server = app.listen(PORT, () => {
  console.log(`Server running on http://localhost:${PORT}`);
  console.log(`GrepDB URL: ${GREPDB_URL}`);
  console.log('\nEndpoints:');
  console.log('  POST /assistant - Send messages to AI assistant with grep capabilities');
  console.log('  POST /update_workflow - Index files in GrepDB');
  console.log('  GET /health - Health check');
});

// Keep the process alive
process.on('SIGINT', () => {
  console.log('\nShutting down gracefully...');
  server.close(() => {
    console.log('Server closed');
    process.exit(0);
  });
});
