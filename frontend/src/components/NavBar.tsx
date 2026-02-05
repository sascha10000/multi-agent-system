import { Link, useLocation } from 'react-router-dom';

function NavBar() {
  const location = useLocation();

  const isActive = (path: string) => {
    if (path === '/') {
      return location.pathname === '/';
    }
    return location.pathname.startsWith(path);
  };

  return (
    <nav className="navbar">
      <div className="navbar-brand">
        <Link to="/">
          <span className="navbar-logo">MAS</span>
          <span className="navbar-title">Multi-Agent System</span>
        </Link>
      </div>
      <div className="navbar-links">
        <Link
          to="/"
          className={`navbar-link ${isActive('/') && location.pathname === '/' ? 'active' : ''}`}
        >
          Dashboard
        </Link>
        <Link
          to="/systems"
          className={`navbar-link ${isActive('/systems') ? 'active' : ''}`}
        >
          Systems
        </Link>
      </div>
    </nav>
  );
}

export default NavBar;
