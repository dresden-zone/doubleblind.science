import { Injectable } from '@angular/core';
import {Observable, of} from "rxjs";
import {Project} from "./project.domain";
import {Repository} from "./repository.domain";

@Injectable({
  providedIn: 'root'
})
export class RepositoryService {

  constructor() { }

  public getRepositories() : Observable<Repository[]> {
    return of([
      {
        name: "dd-ix/nix-config",
        logo: "https://avatars.githubusercontent.com/u/110357347?s=48&v=4",
        repo_id: 3
      },
      {
          name: "tanneberger/bahn.bingo",
          logo: "https://avatars.githubusercontent.com/u/32239737?s=48&v=4",
          repo_id: 4
      },
      {
          name: "tlm-solutions/datacare",
          logo: "https://avatars.githubusercontent.com/u/104242032?s=48&v=4",
          repo_id: 5
      }
    ]);
  }
}
