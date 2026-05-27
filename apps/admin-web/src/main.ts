import './style.css'

const app = document.querySelector<HTMLDivElement>('#app')

if (!app) {
  throw new Error('App root not found')
}

app.innerHTML = `
  <main class="shell">
    <section class="hero">
      <p class="eyebrow">Automated Mine Management System</p>
      <h1>Dispatch, queue, loading and weighing in one operating view.</h1>
      <p class="intro">
        This admin console is the control center for mine transportation scheduling.
        It will host dispatch operations, pit queue monitoring, alerts and live production data.
      </p>
    </section>

    <section class="grid">
      <article class="card">
        <span class="label">Dispatch</span>
        <h2>Waybill flow</h2>
        <p>Create tasks, assign drivers and track each trip through completion.</p>
      </article>

      <article class="card">
        <span class="label">Queue</span>
        <h2>Pit visibility</h2>
        <p>Watch queue length, waiting time and call order for each active pit.</p>
      </article>

      <article class="card">
        <span class="label">Alerts</span>
        <h2>Operational exceptions</h2>
        <p>Flag late arrivals, long loading times and weight deviations in real time.</p>
      </article>
    </section>
  </main>
`
