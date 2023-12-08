import {Component, Input} from '@angular/core';
import { CommonModule } from '@angular/common';
import {ProjectService} from "../../core/data/project.service";
import {IconTrashComponent} from "../../core/icons/icon-trash/icon-trash.component";
import {ButtonComponent, TextFieldComponent, DropdownComponent, OptionComponent} from "@feel/form";
import {CardComponent} from "../../core/components/card/card.component";
import {IconEyeComponent} from "../../core/icons/icon-eye/icon-eye.component";
import {RepositoryService} from "../../core/data/repository.service";
import {FormControl, FormGroup, ReactiveFormsModule, Validators} from "@angular/forms";
import {NotificationService} from "@feel/notification";
import {BehaviorSubject, debounceTime, of, switchMap} from "rxjs";

@Component({
  selector: 'app-projects',
  standalone: true,
  imports: [CommonModule, IconTrashComponent, ButtonComponent, CardComponent, IconEyeComponent, TextFieldComponent, TextFieldComponent, ButtonComponent, DropdownComponent, ReactiveFormsModule, OptionComponent],
  templateUrl: './projects.component.html',
  styleUrl: './projects.component.scss'
})
export class ProjectsComponent {
  protected searchTerm = new BehaviorSubject<string | undefined>(undefined);
  protected readonly projects = this.projectService.getUserProjects();
  protected readonly repositories= this.searchTerm.pipe(
    debounceTime(200),
    switchMap(searchTerm => searchTerm?this.repositoryService.getRepositories(searchTerm):of([]))
  );
  constructor(
    private readonly projectService:ProjectService,
    private readonly repositoryService:RepositoryService,
    private readonly notificationService: NotificationService,
  ) {
    this.form.valueChanges.subscribe(console.log);
  }

  @Input()
  public projectName: string | null = null;

  @Input()
  public projectRepo: number | null = null;


  protected form = new FormGroup( {
    name: new FormControl(null, [Validators.required, Validators.minLength(6)]),
    github_name: new FormControl(null, [Validators.required]),
  })

  protected visit_website(subdomain: string) {
      location.href=`https://${subdomain}.science.tanneberger.me`;
  }

  protected validate_input_and_submit() {
    if (!this.form.valid) {
      console.log("invalid form!");
      return;
    }

    const value = this.form.value;

    this.projectService.create(value.name!, value.github_name!)
      .subscribe({
        next: () => {
          this.notificationService.success(`Successfully Created Project`);
        },
        error: err => {
          console.error(err);
          this.notificationService.error(`Failed to Create Project`);
        },
      });
  }
}
