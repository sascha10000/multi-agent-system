import { Routes, Route } from 'react-router-dom'
import NavBar from './components/NavBar'
import HomePage from './pages/HomePage'
import SystemsPage from './pages/SystemsPage'
import SystemDetailPage from './pages/SystemDetailPage'
import ChatPage from './pages/ChatPage'

function App() {
  return (
    <div className="app">
      <NavBar />
      <main className="main-content">
        <Routes>
          <Route path="/" element={<HomePage />} />
          <Route path="/systems" element={<SystemsPage />} />
          <Route path="/systems/:name" element={<SystemDetailPage />} />
          <Route path="/systems/:name/chat" element={<ChatPage />} />
        </Routes>
      </main>
    </div>
  )
}

export default App
