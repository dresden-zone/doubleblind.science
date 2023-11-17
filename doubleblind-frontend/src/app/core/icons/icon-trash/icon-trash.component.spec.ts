import { ComponentFixture, TestBed } from '@angular/core/testing';

import { IconTrashComponent } from './icon-trash.component';

describe('IconTrashComponent', () => {
  let component: IconTrashComponent;
  let fixture: ComponentFixture<IconTrashComponent>;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [IconTrashComponent]
    });
    fixture = TestBed.createComponent(IconTrashComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
