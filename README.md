# bleeding-edge

Run a Minecraft server that is dangerously up-to-date.

## Environment Variables

| Name                           | Default | Description                                                                                         |
| ------------------------------ | ------- | --------------------------------------------------------------------------------------------------- |
| BLEEDINGEDGE_WORKING_DIRECTORY | run     | The directory the server will be exceuted in                                                        |
| BLEEDINGEDGE_BACKUP_DIRECTORY  | backups | The directory where backups created during transition between Minecraft versions will be stored     |
| BLEEDINGEDGE_USE_AIKAR         | 1       | Should the server be ran with Aikar's JVM flags? Set to something other than 1 (e.g. 0) to disable. |
| BLEEDINGEDGE_MIN_MEM           | 1g      | The minimum size of the JVM heap                                                                    |
| BLEEDINGEDGE_MAX_MEM           | 1g      | The maximum size of the JVM heap                                                                    |
