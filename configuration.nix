{
  services.sonarr.enable = true;
  services.sabnzbd = {
    pkg = pkgs.unstable.sabnzbd;
    enable = true;
  };
  services = {
    radarr = {
      enable = true;
      port = 8080;
    };
  };
  
  systemd.services."sonarr" = {
    enable = true;
  };
}
