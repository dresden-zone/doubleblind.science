{ pkgs, config, lib, ... }:
let
  cfg = config.dresden-zone.doubleblind;
in
{
  options.dresden-zone.doubleblind = with lib; {
    enable = mkOption {
      type = types.bool;
      default = false;
      description = ''Wether to enable doubleblind.science backend'';
    };
    http = {
        host = mkOption {
          type = types.str;
          default = "127.0.0.1";
          description = ''
            ip address of doubleblind
          '';
        };
        port = mkOption {
          type = types.port;
          default = 8080;
          description = ''
            port address of doubleblind
          '';
        };
       };
    database = {
      host = mkOption {
        type = types.str;
        default = "127.0.0.1";
        description = ''
          Database host
        '';
      };
      port = mkOption {
        type = types.port;
        default = 5354;
        description = ''
          Database port
        '';
      };
      user = mkOption {
        type = types.str;
        default = "doubleblind";
        description = ''
          user for postgres
        '';
      };
      database = mkOption {
        type = types.str;
        default = "tlms";
        description = ''
          postgres database to use
        '';
      };
      passwordFile = mkOption {
        type = types.either types.path types.string;
        description = ''password file from which the postgres password can be read'';
      };
    };
    github = {
        clientID = mkOption {
          type = types.str;
          description = ''id of oauth with github'';
        };
         passwordFile = mkOption {
           type = types.either types.path types.string;
           default = "";
           description = ''password file from which the github oauth secret can be read'';
         };
    };

    domain = mkOption {
      type = types.str;
      default = "doubleblind.science";
      description = ''domain under which the websites will be hosted'';
    };

    storageLocation =  mkOption {
      type = types.either types.path types.string;
      default = "/var/lib/doubleblind/sites/";
      description = ''place where the webpages should be dropped'';
    };

    user = mkOption {
      type = types.str;
      default = "doubleblind";
      description = ''systemd user'';
    };
    group = mkOption {
      type = types.str;
      default = "doubleblind";
      description = ''group of systemd user'';
    };
    log_level = mkOption {
      type = types.str;
      default = "info";
      description = ''log level of the application'';
    };
  };

  config = lib.mkIf cfg.enable {
    systemd = {
      services = {
        "doubleblind" = {
          enable = true;
          wantedBy = [ "multi-user.target" "network.target" ];

          script = ''
            exec ${pkgs.doubleblind-backend}/bin/doubeblind-science&
          '';

          environment = {
            #"RUST_LOG" = "${cfg.log_level}";
            #"RUST_BACKTRACE" = if (cfg.log_level == "info") then "0" else "1";
            "DOUBLEBLIND_LISTEN_ADDR" = "${cfg.http.host}:${toString cfg.http.port}";
            "DOUBLEBLIND_POSTGRES_HOST" = "${cfg.database.host}:${toString cfg.database.port}";
            "DOUBLEBLIND_POSTGRES_USERNAME" = "${toString cfg.database.user}";
            "DOUBLEBLIND_POSTGRES_DATABASE_NAME" = "${toString cfg.database.database}";
            "DOUBLEBLIND_POSTGRES_PASSWORD_PATH" = "${cfg.database.passwordFile}";
            "DOUBLEBLIND_GITHUB_CLIENT_ID" = "${cfg.github.clientID}";
            "DOUBLEBLIND_GITHUB_CLIENT_SECRET_PATH" = "${cfg.github.passwordFile}";
            "DOUBLEBLIND_WEBSITE_PATH" = "${cfg.storageLocation}";
            "DOUBLEBLIND_WEBSITE_DOMAIN" = "${cfg.domain}";
          };

          serviceConfig = {
            Type = "forking";
            User = cfg.user;
            Restart = "always";
          };
        };
      };
    };

    # user accounts for systemd units
    users.users."${cfg.user}" = {
      name = "${cfg.user}";
      description = "runs doubleblind";
      isNormalUser = false;
      isSystemUser = true;
      group = cfg.group;
    };

    users.groups."${cfg.group}" = {
      name = "doubleblind";
      members = [ cfg.user ];
    };
  };
}
