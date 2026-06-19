import { useEffect } from "react";

export default function AppsTab({
  filteredApps,
  appsSearch,
  setAppsSearch,
  appsSearchRef,
  onSelect,
}) {
  useEffect(() => {
    appsSearchRef.current?.focus();
  }, [appsSearchRef]);

  return (
    <>
      <div className="search-bar">
        <input
          ref={appsSearchRef}
          className="search-input"
          type="text"
          autoComplete="off"
          autoCorrect="off"
          spellCheck={false}
          placeholder="Search apps..."
          value={appsSearch}
          onChange={(e) => setAppsSearch(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && filteredApps.length > 0) {
              e.preventDefault();
              onSelect(filteredApps[0].id);
            }
            if (e.key === "Escape") {
              e.stopPropagation();
              e.target.blur();
            }
          }}
        />
      </div>
      <div className="list">
        {filteredApps.length === 0 && <div className="empty">No apps</div>}
        {filteredApps.map((app, i) => (
          <div key={app.id} className="item app-item" onClick={() => onSelect(app.id)}>
            <span className="app-index">{i < 9 ? i + 1 : ""}</span>
            <span className="session-name">{app.name}</span>
          </div>
        ))}
      </div>
    </>
  );
}
