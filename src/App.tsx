import { StoreProvider } from './hooks/StoreProvider';
import { AppShell } from './components/AppShell';

export default function App() {
  return <StoreProvider><AppShell /></StoreProvider>;
}
