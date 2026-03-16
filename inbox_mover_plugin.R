#!/usr/bin/env Rscript
#' Inbox Mover Plugin
#' ------------------
#' A utility to process, extract, and log ZIP files (and standard files) from a specific source folder.
#' 
#' 🌱 ALPHA RELEASE NOTICE:
#'   This plugin is currently in its early alpha stage! While it has been 
#'   crafted with care, please do not trust it blindly just yet. We warmly 
#'   recommend testing it thoroughly in a safe, backed-up environment to 
#'   ensure it does exactly what you need before letting it loose on your 
#'   important files.
#' 
#' USAGE AS A PLUGIN:
#'   source("inbox_mover_plugin.R")
#'   
#'   config <- list(
#'     target_folder = "/path/to/extract/files",
#'     processed_folder = "/path/to/move/completed/source",
#'     receipt_folder = "/path/to/save/receipts",
#'     conflict_resolution = "rename_existing",
#'     post_processing = "move"
#'   )
#'   
#'   result <- im("/path/to/source_folder", config)
#'   print(result)
#' 
#' USAGE FROM COMMAND LINE:
#'   Rscript inbox_mover_plugin.R --source "C:/test/source_folder" --config "C:/test/config.json"
#' 
#' CONFIG DICTIONARY / JSON STRUCTURE:
#'   Same as Python version. All options apply.

# Ensure required packages are installed/loaded
suppressPackageStartupMessages({
  if (!requireNamespace("R6", quietly = TRUE)) install.packages("R6", repos = "http://cran.us.r-project.org")
  if (!requireNamespace("jsonlite", quietly = TRUE)) install.packages("jsonlite", repos = "http://cran.us.r-project.org")
  if (!requireNamespace("optparse", quietly = TRUE)) install.packages("optparse", repos = "http://cran.us.r-project.org")
  library(R6)
  library(jsonlite)
  library(optparse)
})

#' Helper to safely get list values with defaults
get_val <- function(lst, key, default = NULL) {
  if (key %in% names(lst) && !is.null(lst[[key]])) lst[[key]] else default
}

InboxProcessor <- R6Class("InboxProcessor",
  public = list(
    base_config = NULL,
    actions_log = list(),
    
    initialize = function(config) {
      self$base_config <- list(
        target_folder = get_val(config, "target_folder"),
        processed_folder = get_val(config, "processed_folder"),
        receipt_folder = get_val(config, "receipt_folder"),
        conflict_resolution = get_val(config, "conflict_resolution", "overwrite"),
        post_processing = get_val(config, "post_processing", "leave")
      )
      self$actions_log <- list()
    },
    
    process = function(source_folder) {
      source_folder <- normalizePath(source_folder, mustWork = FALSE)
      
      if (!file.exists(source_folder)) {
        return(private$fail(sprintf("Source folder does not exist: %s", source_folder)))
      }
      if (!dir.exists(source_folder)) {
        return(private$fail(sprintf("Source path is not a directory: %s", source_folder)))
      }
      
      # 1. Inspect the folder
      folder_data <- private$inspect_folder(source_folder)
      if (!folder_data$can_process) {
        return(private$fail("Folder is empty. Nothing to process.", folder_data))
      }
      
      # 2. Merge config with any overrides from a discovered receipt.json
      active_config <- private$merge_config(folder_data$receipt)
      
      # 3. Validate configuration
      if (is.null(active_config$target_folder) || active_config$target_folder == "") {
        return(private$fail("Target folder is not specified in config or receipt.", folder_data, active_config))
      }
      
      if (active_config$post_processing == "move" && (is.null(active_config$processed_folder) || active_config$processed_folder == "")) {
        return(private$fail("Post processing is 'move' but 'processed_folder' is not specified.", folder_data, active_config))
      }
      
      # 4. Execute file operations
      result <- tryCatch({
        private$ensure_directories(active_config)
        private$process_files(folder_data, active_config)
        private$apply_post_processing(folder_data, active_config)
        
        # Write logs to the final destination of the folder/files
        private$write_local_log("SUCCESS", folder_data, active_config, "Successfully processed folder.")
        private$update_receipt_log(folder_data, active_config)
        
        list(
          status = "SUCCESS",
          message = "Processed successfully.",
          actions = self$actions_log,
          config_used = active_config
        )
      }, error = function(e) {
        private$write_local_log("ERROR", folder_data, active_config, e$message)
        private$fail(sprintf("Processing error: %s", e$message), folder_data, active_config)
      })
      
      return(result)
    }
  ),
  
  private = list(
    fail = function(message, folder_data = NULL, config = NULL) {
      cfg_to_use <- if (!is.null(config)) config else self$base_config
      if (!is.null(folder_data)) {
        private$write_local_log("ERROR", folder_data, cfg_to_use, message)
      }
      return(list(
        status = "ERROR",
        message = message,
        actions = self$actions_log,
        config_used = cfg_to_use
      ))
    },
    
    inspect_folder = function(folder_path) {
      data <- list(
        folder_path = folder_path,
        folder_name = basename(folder_path),
        zip_path = NULL,
        permitId = "DEFAULT",
        receipt = NULL,
        has_valid_zip = FALSE,
        can_process = FALSE
      )
      
      files <- list.files(folder_path, recursive = TRUE, full.names = TRUE)
      
      for (f in files) {
        if (grepl("\\.zip$", f, ignore.case = TRUE)) {
          zip_info <- private$inspect_zip(f)
          if (!is.null(zip_info) && !is.null(zip_info$receipt)) {
            data$zip_path <- f
            data$permitId <- zip_info$permitId
            data$receipt <- zip_info$receipt
            data$has_valid_zip <- TRUE
            data$can_process <- TRUE
            return(data)
          }
        }
      }
      
      # Fallback: No valid zip with receipt found. Check if folder has files.
      if (length(files) > 0) {
        data$can_process <- TRUE
      }
      return(data)
    },
    
    inspect_zip = function(zip_path) {
      # List files inside zip without fully extracting them yet
      files_in_zip <- tryCatch(utils::unzip(zip_path, list = TRUE)$Name, error = function(e) NULL)
      if (is.null(files_in_zip)) return(NULL)
      
      receipt_filename <- files_in_zip[grepl("receipt\\.json$", files_in_zip, ignore.case = TRUE)]
      
      if (length(receipt_filename) > 0) {
        receipt_filename <- receipt_filename[1]
        temp_dir <- file.path(tempdir(), paste0("zip_inspect_", as.numeric(Sys.time())))
        dir.create(temp_dir, showWarnings = FALSE)
        
        tryCatch({
          utils::unzip(zip_path, files = receipt_filename, exdir = temp_dir)
          receipt_path <- file.path(temp_dir, receipt_filename)
          
          if (file.exists(receipt_path)) {
            receipt_json <- jsonlite::fromJSON(receipt_path, simplifyVector = FALSE)
            permitId <- get_val(receipt_json, "permitId", "DEFAULT")
            unlink(temp_dir, recursive = TRUE)
            return(list(receipt = receipt_json, permitId = permitId))
          }
        }, error = function(e) {
          unlink(temp_dir, recursive = TRUE)
        })
      }
      return(NULL)
    },
    
    merge_config = function(receipt_data) {
      merged <- self$base_config
      if (is.null(receipt_data) || length(receipt_data) == 0) return(merged)
      
      override_mapping <- list(
        target_folder = "target_folder",
        processed_folder = "processed_folder",
        process_folder = "processed_folder", # legacy
        receipt_folder = "receipt_folder",
        conflict_resolution = "conflict_resolution",
        post_processing = "post_processing"
      )
      
      for (receipt_key in names(override_mapping)) {
        config_key <- override_mapping[[receipt_key]]
        if (!is.null(receipt_data[[receipt_key]])) {
          merged[[config_key]] <- receipt_data[[receipt_key]]
        }
      }
      return(merged)
    },
    
    ensure_directories = function(config) {
      if (!is.null(config$target_folder)) dir.create(config$target_folder, recursive = TRUE, showWarnings = FALSE)
      if (!is.null(config$receipt_folder)) dir.create(config$receipt_folder, recursive = TRUE, showWarnings = FALSE)
      if (config$post_processing == "move" && !is.null(config$processed_folder)) {
        dir.create(config$processed_folder, recursive = TRUE, showWarnings = FALSE)
      }
    },
    
    get_final_path = function(extracted_path, conflict_res) {
      if (!file.exists(extracted_path)) return(extracted_path)
      
      if (conflict_res == 'overwrite') {
        self$actions_log <- append(self$actions_log, list(list(
          type = "conflict_resolved",
          source = extracted_path,
          message = "Existing file overwritten"
        )))
        return(extracted_path)
        
      } else if (conflict_res == 'keep_both') {
        dir_name <- dirname(extracted_path)
        base_name <- tools::file_path_sans_ext(basename(extracted_path))
        ext <- tools::file_ext(extracted_path)
        ext_dot <- ifelse(nchar(ext) > 0, paste0(".", ext), "")
        
        counter <- 1
        new_path <- file.path(dir_name, paste0(base_name, " (", counter, ")", ext_dot))
        while (file.exists(new_path)) {
          counter <- counter + 1
          new_path <- file.path(dir_name, paste0(base_name, " (", counter, ")", ext_dot))
        }
        
        self$actions_log <- append(self$actions_log, list(list(
          type = "conflict_resolved",
          source = extracted_path,
          message = sprintf("Kept both. Extracted file renamed to %s", basename(new_path))
        )))
        return(new_path)
        
      } else if (conflict_res == 'rename_existing') {
        timestamp <- format(Sys.time(), "%y%m%d-%H%M%S")
        dir_name <- dirname(extracted_path)
        filename <- basename(extracted_path)
        renamed_path <- file.path(dir_name, paste0(timestamp, "_", filename))
        
        if (file.exists(renamed_path)) {
          counter <- 1
          while (file.exists(paste0(renamed_path, "_", counter))) {
            counter <- counter + 1
          }
          renamed_path <- paste0(renamed_path, "_", counter)
        }
        
        file.rename(extracted_path, renamed_path)
        self$actions_log <- append(self$actions_log, list(list(
          type = "conflict_resolved",
          source = extracted_path,
          message = sprintf("Existing file renamed to %s", basename(renamed_path))
        )))
        return(extracted_path)
      }
      return(extracted_path)
    },
    
    process_files = function(folder_data, config) {
      target_folder <- config$target_folder
      receipt_folder <- config$receipt_folder
      conflict_res <- config$conflict_resolution
      
      extract_zip_routine <- function(zip_path) {
        zip_filename <- basename(zip_path)
        temp_ex <- file.path(tempdir(), paste0("zip_ext_", as.numeric(Sys.time())))
        dir.create(temp_ex, recursive = TRUE, showWarnings = FALSE)
        
        utils::unzip(zip_path, exdir = temp_ex)
        extracted_files <- list.files(temp_ex, recursive = TRUE, full.names = TRUE)
        
        for (src_path in extracted_files) {
          # Construct relative path
          original_name <- sub(paste0("^", temp_ex, "[/\\]?"), "", src_path)
          
          if (grepl("receipt\\.json$", original_name, ignore.case = TRUE)) {
            timestamp <- format(Sys.time(), "%y%m%d-%H%M%S")
            new_filename <- paste0(timestamp, "-", basename(original_name))
            
            if (!is.null(receipt_folder) && dir.exists(receipt_folder)) {
              ext_path <- file.path(receipt_folder, new_filename)
            } else {
              ext_path <- file.path(target_folder, dirname(original_name), new_filename)
            }
          } else {
            ext_path <- file.path(target_folder, original_name)
          }
          
          dir.create(dirname(ext_path), recursive = TRUE, showWarnings = FALSE)
          final_path <- private$get_final_path(ext_path, conflict_res)
          
          file.copy(src_path, final_path, overwrite = TRUE)
          
          self$actions_log <- append(self$actions_log, list(list(
            type = "extract",
            source = paste(zip_filename, "->", original_name),
            destination = final_path
          )))
        }
        unlink(temp_ex, recursive = TRUE)
      }
      
      # Logic fork: process zip or loose files
      if (folder_data$has_valid_zip && !is.null(folder_data$zip_path)) {
        extract_zip_routine(folder_data$zip_path)
      } else {
        folder_path <- folder_data$folder_path
        folder_name <- folder_data$folder_name
        
        src_files <- list.files(folder_path, recursive = TRUE, full.names = TRUE)
        for (src_path in src_files) {
          if (grepl("\\.zip$", src_path, ignore.case = TRUE)) {
            extract_zip_routine(src_path)
          } else {
            rel_path <- sub(paste0("^", folder_path, "[/\\]?"), "", src_path)
            ext_path <- file.path(target_folder, rel_path)
            
            dir.create(dirname(ext_path), recursive = TRUE, showWarnings = FALSE)
            final_path <- private$get_final_path(ext_path, conflict_res)
            
            file.copy(src_path, final_path, overwrite = TRUE)
            self$actions_log <- append(self$actions_log, list(list(
              type = "copy",
              source = paste(folder_name, "->", rel_path),
              destination = final_path
            )))
          }
        }
      }
    },
    
    apply_post_processing = function(folder_data, config) {
      post_action <- config$post_processing
      folder_path <- folder_data$folder_path
      
      if (post_action == 'delete') {
        if (!is.null(folder_path) && dir.exists(folder_path)) {
          unlink(folder_path, recursive = TRUE, force = TRUE)
          self$actions_log <- append(self$actions_log, list(list(
            type = "post_processing",
            source = folder_path,
            destination = "DELETED"
          )))
        }
      } else if (post_action == 'move') {
        processed_folder <- config$processed_folder
        folder_name <- folder_data$folder_name
        dest_path <- file.path(processed_folder, folder_name)
        
        if (dir.exists(dest_path) || file.exists(dest_path)) {
          counter <- 1
          while (dir.exists(paste0(dest_path, "_", counter)) || file.exists(paste0(dest_path, "_", counter))) {
            counter <- counter + 1
          }
          dest_path <- paste0(dest_path, "_", counter)
        }
        
        # R's file.rename can fail across devices. Using copy + unlink is safer for moving directories.
        file.copy(folder_path, processed_folder, recursive = TRUE)
        if (basename(dest_path) != folder_name) {
           file.rename(file.path(processed_folder, folder_name), dest_path)
        }
        unlink(folder_path, recursive = TRUE, force = TRUE)
        
        self$actions_log <- append(self$actions_log, list(list(
          type = "post_processing",
          source = folder_path,
          destination = dest_path
        )))
      }
    },
    
    write_local_log = function(status, folder_data, config, message = "") {
      post_action <- get_val(config, 'post_processing', 'leave')
      target_local_dir <- NULL
      
      if (post_action == 'leave') {
        target_local_dir <- folder_data$folder_path
      } else if (post_action == 'move') {
        for (act in self$actions_log) {
          if (get_val(act, "type") == "post_processing" && get_val(act, "destination") != "DELETED") {
            target_local_dir <- act$destination
            break
          }
        }
      } else if (post_action == 'delete' && status != 'SUCCESS') {
        target_local_dir <- folder_data$folder_path
      }
      
      if (is.null(target_local_dir) || !dir.exists(target_local_dir)) return()
      
      local_log_path <- file.path(target_local_dir, "Inbox Process.log")
      ts <- format(Sys.time(), "%Y-%m-%d %H:%M:%S")
      user <- Sys.info()[["user"]]
      permit_id <- get_val(folder_data, 'permitId', 'Unknown')
      folder_name <- get_val(folder_data, 'folder_name', 'Unknown')
      
      lines <- c(sprintf("[%s] %s | User: %s | Config: %s | Folder: %s", ts, status, user, permit_id, folder_name))
      
      if (nchar(message) > 0) lines <- c(lines, sprintf("  Message: %s", message))
      
      if (length(self$actions_log) > 0) {
        lines <- c(lines, "  Actions:")
        for (act in self$actions_log) {
          a_type <- toupper(get_val(act, "type", ""))
          a_src <- get_val(act, "source", "")
          a_dest <- get_val(act, "destination", "")
          a_msg <- get_val(act, "message", "")
          
          if (a_type == "CONFLICT_RESOLVED") {
            lines <- c(lines, sprintf("    - CONFLICT: %s -> %s", a_src, a_msg))
          } else {
            lines <- c(lines, sprintf("    - %s: %s -> %s", a_type, a_src, a_dest))
          }
        }
      }
      
      lines <- c(lines, paste(rep("-", 80), collapse = ""))
      new_log_text <- paste(lines, collapse = "\n")
      new_log_text <- paste0(new_log_text, "\n\n")
      
      existing_content <- ""
      if (file.exists(local_log_path)) {
        tryCatch({
          existing_content <- readChar(local_log_path, file.info(local_log_path)$size)
        }, error = function(e) {})
      }
      
      tryCatch({
        cat(paste0(new_log_text, existing_content), file = local_log_path, append = FALSE)
      }, error = function(e) {
        cat(sprintf("Failed to write local log to %s: %s\n", local_log_path, e$message))
      })
    },
    
    update_receipt_log = function(folder_data, config) {
      log_entry <- list(
        timestamp = format(Sys.time(), "%Y-%m-%dT%H:%M:%S"),
        user = Sys.info()[["user"]],
        status = "SUCCESS",
        folder_name = get_val(folder_data, 'folder_name', 'Unknown'),
        config_id = get_val(folder_data, 'permitId', 'Unknown'),
        files_processed = sum(sapply(self$actions_log, function(a) get_val(a, "type") %in% c('extract', 'copy'))),
        config_applied = config,
        actions = self$actions_log
      )
      
      for (act in self$actions_log) {
        dest <- get_val(act, "destination", "")
        if (is.character(dest) && grepl("receipt\\.json$", dest, ignore.case = TRUE) && file.exists(dest)) {
          tryCatch({
            receipt_data <- tryCatch({
              jsonlite::fromJSON(dest, simplifyVector = FALSE)
            }, error = function(e) list(`_raw_file_unparsable` = TRUE))
            
            if (is.null(receipt_data$processing_logs)) {
              receipt_data$processing_logs <- list()
            }
            
            # Append new log
            receipt_data$processing_logs <- append(receipt_data$processing_logs, list(log_entry))
            
            jsonlite::write_json(receipt_data, dest, auto_unbox = TRUE, pretty = TRUE)
          }, error = function(e) {
            cat(sprintf("Failed to update receipt.json with processing log: %s\n", e$message))
          })
        }
      }
    }
  )
)

#' Main plugin entry point
#' 
#' @param source_folder A string path to the directory to process.
#' @param config A list containing the configuration keys.
#' @return A list containing processing status and action logs.
im <- function(source_folder, config) {
  processor <- InboxProcessor$new(config)
  return(processor$process(source_folder))
}

# --------------------------------------------------------------------------- #
# CLI IMPLEMENTATION
# --------------------------------------------------------------------------- #

# Check if script is being run directly via Rscript
if (!interactive() && identical(environment(), globalenv())) {
  
  option_list <- list(
    make_option(c("-s", "--source"), type="character", default=NULL, 
                help="Path to the source folder containing the zip/files.", metavar="character"),
    make_option(c("-c", "--config"), type="character", default=NULL, 
                help="Path to the JSON configuration file.", metavar="character")
  )
  
  opt_parser <- OptionParser(
    option_list=option_list, 
    description="Inbox Mover Plugin CLI",
    epilogue="
🌱 ALPHA RELEASE NOTICE:
  This plugin is currently in its early alpha stage! Please test it 
  thoroughly in a safe environment to ensure it meets your needs 
  before using it on critical files.

Example Usage:
  Rscript inbox_mover_plugin.R --source \"C:/transfer-123\" --config \"C:/config.json\"
  
Config JSON Structure:
  {
      \"target_folder\": \"C:/output/extracted\",
      \"processed_folder\": \"C:/output/processed_zips\",
      \"receipt_folder\": \"C:/output/receipts\",
      \"conflict_resolution\": \"rename_existing\",
      \"post_processing\": \"move\"
  }
"
  )
  
  opt <- parse_args(opt_parser)
  
  if (is.null(opt$source) || is.null(opt$config)) {
    print_help(opt_parser)
    quit(status=1)
  }
  
  if (!file.exists(opt$config)) {
    cat(sprintf("Error: Config file not found at %s\n", opt$config))
    quit(status=1)
  }
  
  plugin_config <- tryCatch({
    jsonlite::fromJSON(opt$config, simplifyVector = FALSE)
  }, error = function(e) {
    cat(sprintf("Error parsing config JSON file: %s\n", e$message))
    quit(status=1)
  })
  
  cat(sprintf("Processing Source: %s\n...\n", opt$source))
  
  result <- im(opt$source, plugin_config)
  
  if (result$status == "SUCCESS") {
    cat(sprintf("SUCCESS: %s\n", result$message))
    extracted_count <- sum(sapply(result$actions, function(a) get_val(a, "type") %in% c("extract", "copy")))
    cat(sprintf("Files extracted/copied: %d\n", extracted_count))
  } else {
    cat(sprintf("ERROR: %s\n", result$message))
  }
  
  if (length(result$actions) > 0) {
    for (action in result$actions) {
      a_type <- toupper(get_val(action, "type", ""))
      a_src <- get_val(action, "source", "")
      a_dest <- get_val(action, "destination", get_val(action, "message", ""))
      cat(sprintf("  - %s: %s -> %s\n", a_type, a_src, a_dest))
    }
  }
}