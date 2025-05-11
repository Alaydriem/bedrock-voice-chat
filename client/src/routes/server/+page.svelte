<script lang="ts">
    // Query all servers from the stronghold servers file

    // If there are none, redirect to the login page.

    // If there's one, ping the server
        // If it's good with the credentials, redirect to the dashboard
        // If it's bad, clear the login from the server, then redirect to login with the uri pre-populated, then immediately trigger the login flow
    
    // If there's more than one server
        // Show icons for all of them, fetched from the servers
        // The ping all of them and show the ones that are connectable in color vs b/w
        // If the user clicks on a colored one
            // Go to the dashboard with the current server set
        
        // If the user clicks on a b/w one
            // Redirect to the login page with the domain pre-populated, then immediately jump to the login form
    window.location.href="/login";
</script>