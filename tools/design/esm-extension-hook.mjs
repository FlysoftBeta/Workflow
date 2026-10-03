// material-color-utilities 0.4.0 ships a few extensionless relative ESM imports
// (e.g. '../dynamiccolor/dynamic_scheme'), which Node's ESM loader rejects.
// This resolve hook retries such specifiers with a '.js' suffix; nothing else is changed.
import { register } from 'node:module';

register('data:text/javascript,' + encodeURIComponent(`
export async function resolve(specifier, context, next) {
  try {
    return await next(specifier, context);
  } catch (error) {
    if (error && error.code === 'ERR_MODULE_NOT_FOUND' && specifier.startsWith('.') && !specifier.endsWith('.js')) {
      return next(specifier + '.js', context);
    }
    throw error;
  }
}`));
